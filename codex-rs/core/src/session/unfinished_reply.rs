//! Fikra: keep a turn going when the model stops on an announcement.
//!
//! Some models (DeepSeek in particular) end a long turn with text such as
//! "21/21. Let me run the whole suite..." and no tool call. The loop treats
//! any reply without a tool call as final, so the turn ends on a half step
//! and the user never gets a summary. When the final reply is empty or its
//! last sentence announces more work, the turn is continued with a short
//! prompt, at most [`MAX_AUTO_CONTINUES`] times per turn (long runs can stall several times).

pub(crate) const MAX_AUTO_CONTINUES: u8 = 6;

pub(crate) const CONTINUE_PROMPT: &str = "You stopped before finishing: your last message announced a next step but no tool was called. \
Continue the work now. When everything is done, end with a short final summary of what changed, what you verified, and anything left for the user.";

pub(crate) const HOOK_RUN_ID: &str = "fikra-auto-continue";

const INTENT_PREFIXES: &[&str] = &[
    "let me ",
    "let's ",
    "i'll ",
    "i will ",
    "i'm going to ",
    "i am going to ",
    "now i'll ",
    "now i will ",
    "now let me ",
    "now let's ",
    "next, i",
    "next i",
    "next, let",
    "next let",
    "then i'll ",
    "first, let",
    "first let",
    "going to ",
];

/// A final sentence that starts with one of these announces work in progress
/// ("Fixing my CN row to match.", "Checking the remaining tests.").
const GERUND_STARTS: &[&str] = &[
    "fixing", "checking", "running", "rerunning", "re-running", "updating", "adding",
    "looking", "trying", "writing", "reading", "creating", "testing", "verifying", "applying",
    "moving", "searching", "inspecting", "building", "installing", "editing", "patching",
    "investigating", "implementing", "starting", "continuing", "wiring", "refactoring",
    "removing", "cleaning", "switching", "rewriting", "retrying", "fetching", "opening",
    "loading", "generating", "converting", "porting", "replacing", "renaming", "deleting",
    "posting", "reproducing", "resuming", "confirming", "pulling", "pushing", "committing",
    "deploying", "scanning", "measuring", "comparing", "downloading", "uploading", "sending",
    "merging", "profiling", "debugging", "tracing", "grepping", "listing", "drafting",
];

const INTENT_VERBS: &[&str] = &[
    "run", "rerun", "re-run", "check", "verify", "fix", "update", "look", "start", "continue",
    "test", "read", "write", "add", "create", "make", "implement", "inspect", "search", "open",
    "try", "build", "commit", "push", "apply", "edit", "review", "confirm", "finish", "clean",
    "install", "compile", "examine", "investigate", "debug", "patch", "refactor", "move",
    "rename", "delete", "remove", "now", "also", "do", "see", "find", "grep", "list", "go",
];

/// The last sentence of `text`, lowercased and trimmed.
fn last_sentence(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .last()
        .unwrap_or("");
    let line = line.trim_start_matches(['-', '*', '>', ' ']);
    // Split on sentence ends, keep the last non-empty piece.
    let mut last = line;
    for piece in line.split_inclusive(['.', '!', '?']) {
        if !piece.trim().is_empty() {
            last = piece;
        }
    }
    last.trim().to_lowercase()
}

/// True when a final reply looks like the model stopped mid-task.
pub(crate) fn looks_unfinished(reply: Option<&str>) -> bool {
    let text = reply.unwrap_or("").trim();
    if text.is_empty() {
        return true;
    }
    if text.ends_with(':') || text.ends_with("...") || text.ends_with('\u{2026}') {
        return true;
    }
    let last = last_sentence(text);
    if last.starts_with("let me know") || last.contains("if you want") || last.contains("if you'd like") {
        return false;
    }
    if last.ends_with('?') {
        return false;
    }
    let mut words = last
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|w| !w.is_empty());
    let mut first_word = words.next().unwrap_or("");
    if matches!(first_word, "now" | "next" | "then" | "so") {
        first_word = words.next().unwrap_or("");
    }
    if GERUND_STARTS.contains(&first_word) {
        return true;
    }
    // "They look suspicious - let me check abuse signals before emailing them."
    for marker in [" let me ", "\u{2014} let me ", ", let me ", "; let me "] {
        if let Some(idx) = last.find(marker) {
            let rest = &last[idx + marker.len()..];
            let verb = rest.split(|c: char| !c.is_alphanumeric() && c != '-').next().unwrap_or("");
            if verb != "know" && INTENT_VERBS.contains(&verb) {
                return true;
            }
        }
    }
    for prefix in INTENT_PREFIXES {
        if let Some(rest) = last.strip_prefix(prefix) {
            let rest = rest.trim_start_matches(|c: char| !c.is_alphanumeric());
            let first = rest.split(|c: char| !c.is_alphanumeric() && c != '-').next().unwrap_or("");
            // "next, i" / "first, let" prefixes already imply the next word.
            if prefix.ends_with(' ') {
                return INTENT_VERBS.contains(&first);
            }
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::looks_unfinished;

    #[test]
    fn catches_announced_next_step() {
        assert!(looks_unfinished(Some(
            "21/21. Let me run the whole suite plus check for regressions in the untouched CLI commands."
        )));
        assert!(looks_unfinished(Some("Tests pass.\n\nNow I'll update the README.")));
        assert!(looks_unfinished(Some("Next, I need to wire the handler.")));
        assert!(looks_unfinished(Some("Here is what I'll change:")));
        assert!(looks_unfinished(Some("")));
        assert!(looks_unfinished(None));
        assert!(looks_unfinished(Some(
            "Good: `facts.qwen_free_status()` is the template pattern. Fixing my CN row to match."
        )));
        assert!(looks_unfinished(Some(
            "`qwen_free_active` isn't a template global. Let me check for an existing helper."
        )));
        assert!(looks_unfinished(Some("Now let me check the test files that pin pricing behavior.")));
        assert!(looks_unfinished(Some("Running the full suite now...")));
        assert!(looks_unfinished(Some("Cached hits bill through the pool. Posting the correction.")));
        assert!(looks_unfinished(Some("Verified. Now confirming the live key context picks up enterprise.")));
        assert!(looks_unfinished(Some("Found all three. They look suspicious, let me check abuse signals first.")));
        assert!(looks_unfinished(Some("42 GB free, disk problem solved. Resuming the build check.")));
    }

    #[test]
    fn leaves_real_summaries_alone() {
        assert!(!looks_unfinished(Some(
            "Done. Added pagination to /orders and updated 3 tests; all 14 pass."
        )));
        assert!(!looks_unfinished(Some("Fixed the bug.\n\nLet me know if you want the README updated too.")));
        assert!(!looks_unfinished(Some("I'll leave the config as it is, since it already matches.")));
        assert!(!looks_unfinished(Some("Summary:\n- fixed X\n- tests pass")));
        assert!(!looks_unfinished(Some("Should I also update the README?")));
        assert!(!looks_unfinished(Some("Fixed the timezone bug; all 14 tests pass.")));
    }
}
