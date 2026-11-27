//! `/budget`: a spend cap the user sets, so a long session cannot quietly cost
//! more than they expect.
//!
//! Spend is estimated from the thread cost the app server reports. The tracker
//! is deliberately small: it holds a cap, the spend observed so far, and whether
//! the warning was already shown.

use std::path::Path;

/// Config file under `FIKRA_HOME`.
const BUDGET_FILE: &str = "budget.toml";

/// Fraction of the cap at which the user is warned before work stops.
pub(crate) const WARN_FRACTION: f64 = 0.8;

/// A spend cap for one session, in US dollars.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Budget {
    /// `None` means no cap.
    pub session_usd: Option<f64>,
    pub daily_usd: Option<f64>,
    /// Model to fall back to when the cap is reached. `None` means stop.
    pub fallback_model: Option<String>,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            session_usd: None,
            daily_usd: None,
            fallback_model: Some("qwen3.8-27b-free".to_string()),
        }
    }
}

/// What the tracker observed about one spend amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BudgetState {
    /// No cap configured, so nothing to enforce.
    Uncapped,
    /// Under the warning threshold.
    Ok,
    /// At or above the warning threshold but below the cap.
    Warned,
    /// At or above the cap.
    Exceeded,
}

impl Budget {
    /// Classifies spend against the session cap.
    pub(crate) fn state(&self, spent_usd: f64) -> BudgetState {
        let Some(cap) = self.session_usd else {
            return BudgetState::Uncapped;
        };
        if cap <= 0.0 {
            return BudgetState::Exceeded;
        }
        if spent_usd >= cap {
            BudgetState::Exceeded
        } else if spent_usd >= cap * WARN_FRACTION {
            BudgetState::Warned
        } else {
            BudgetState::Ok
        }
    }

    /// Percentage of the session cap used, clamped to 0..=100.
    pub(crate) fn percent(&self, spent_usd: f64) -> Option<f64> {
        let cap = self.session_usd?;
        if cap <= 0.0 {
            return Some(100.0);
        }
        Some((spent_usd / cap * 100.0).clamp(0.0, 100.0))
    }

    /// The one-line status shown by `/budget`.
    pub(crate) fn status_line(&self, spent_usd: f64) -> String {
        let Some(cap) = self.session_usd else {
            return format!(
                "No budget set. Spent ${spent_usd:.4} this session. \
                 Set one with `/budget 0.50` or `session_usd = 0.50` in ~/.fikra/budget.toml."
            );
        };
        let percent = self.percent(spent_usd).unwrap_or(0.0);
        let action = match self.state(spent_usd) {
            BudgetState::Exceeded => match &self.fallback_model {
                Some(model) => format!("budget reached; switching to {model}"),
                None => "budget reached; stopping this session".to_string(),
            },
            BudgetState::Warned => format!("${:.2} of ${cap:.2} used", cap - spent_usd),
            _ => format!("${:.2} of ${cap:.2} left", cap - spent_usd),
        };
        format!(
            "Budget ${cap:.2} · spent ${spent_usd:.4} ({percent:.0}%) · {action}"
        )
    }
}

/// Parses `/budget` arguments: empty shows status, a number sets the cap,
/// `off` clears it.
pub(crate) fn parse_setting(args: &str) -> Option<Result<Option<f64>, String>> {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return None;
    }
    if matches!(trimmed, "off" | "none" | "clear") {
        return Some(Ok(None));
    }
    let value = trimmed.trim_start_matches('$');
    match value.parse::<f64>() {
        Ok(amount) if amount > 0.0 && amount.is_finite() => Some(Ok(Some(amount))),
        Ok(_) => Some(Err(format!(
            "`{trimmed}` is not a positive amount; use e.g. `/budget 0.50`"
        ))),
        Err(_) => Some(Err(format!(
            "`{trimmed}` is not a number; use e.g. `/budget 0.50` or `/budget off`"
        ))),
    }
}

/// Loads the cap from `FIKRA_HOME/budget.toml`, falling back to defaults.
pub(crate) fn load(codex_home: &Path) -> Budget {
    let path = codex_home.join(BUDGET_FILE);
    let Ok(body) = std::fs::read_to_string(path) else {
        return Budget::default();
    };
    let mut budget = Budget::default();
    for line in body.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"');
        match key.trim() {
            "session_usd" => budget.session_usd = value.parse().ok(),
            "daily_usd" => budget.daily_usd = value.parse().ok(),
            "fallback_model" => {
                budget.fallback_model = (!value.is_empty()).then(|| value.to_string());
            }
            _ => {}
        }
    }
    budget
}

/// Persists the session cap, preserving the other keys already in the file.
pub(crate) fn save(codex_home: &Path, budget: &Budget) -> std::io::Result<()> {
    let path = codex_home.join(BUDGET_FILE);
    let mut body = String::from("# Fikra Code spend cap. Written by /budget.\n");
    if let Some(amount) = budget.session_usd {
        body.push_str(&format!("session_usd = {amount}\n"));
    }
    if let Some(amount) = budget.daily_usd {
        body.push_str(&format!("daily_usd = {amount}\n"));
    }
    if let Some(model) = &budget.fallback_model {
        body.push_str(&format!("fallback_model = \"{model}\"\n"));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, body)
}

#[cfg(test)]
mod tests {
    use super::Budget;
    use super::BudgetState;
    use super::load;
    use super::parse_setting;
    use super::save;

    fn capped(amount: f64) -> Budget {
        Budget {
            session_usd: Some(amount),
            ..Budget::default()
        }
    }

    #[test]
    fn warns_at_eighty_percent_and_stops_at_the_cap() {
        let budget = capped(1.0);
        assert_eq!(budget.state(0.10), BudgetState::Ok);
        assert_eq!(budget.state(0.80), BudgetState::Warned);
        assert_eq!(budget.state(0.99), BudgetState::Warned);
        assert_eq!(budget.state(1.0), BudgetState::Exceeded);
    }

    #[test]
    fn no_cap_never_reports_a_limit() {
        let budget = Budget::default();
        assert_eq!(budget.state(1000.0), BudgetState::Uncapped);
        assert!(budget.status_line(1000.0).contains("No budget set"));
    }

    #[test]
    fn parses_amount_off_and_rejects_junk() {
        assert_eq!(parse_setting(""), None);
        assert_eq!(parse_setting("0.5"), Some(Ok(Some(0.5))));
        assert_eq!(parse_setting("$2"), Some(Ok(Some(2.0))));
        assert_eq!(parse_setting("off"), Some(Ok(None)));
        assert!(matches!(parse_setting("abc"), Some(Err(_))));
        assert!(matches!(parse_setting("0"), Some(Err(_))));
        assert!(matches!(parse_setting("-1"), Some(Err(_))));
    }

    #[test]
    fn round_trips_through_the_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let budget = Budget {
            session_usd: Some(0.25),
            daily_usd: Some(3.0),
            fallback_model: Some("qwen3.8-27b-free".to_string()),
        };
        save(dir.path(), &budget).expect("save");
        assert_eq!(load(dir.path()), budget);
    }

    #[test]
    fn status_line_reports_fallback_when_exceeded() {
        let budget = capped(0.01);
        let line = budget.status_line(0.02);
        assert!(line.contains("switching to qwen3.8-27b-free"), "{line}");
    }
}
