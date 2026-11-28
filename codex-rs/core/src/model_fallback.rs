//! Choosing a fallback model when a turn hits a rate or quota limit.
//!
//! This module only decides *which* model to move to; the caller performs the
//! switch. Keeping the decision pure makes it testable without a network.

use codex_protocol::error::CodexErr;
use codex_protocol::error::CodexErrorDetails;

/// A model the user can fall back to, in preference order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FallbackCandidate {
    pub slug: String,
    /// True for models that do not bill: preferable when the limit is quota.
    pub free: bool,
}

/// Picks the next model to try after `current` hit a limit.
///
/// Only rate and quota limits are worth switching for: an auth failure would
/// fail identically on every model, so it must not silently move the user.
pub(crate) fn fallback_for_error<'a>(
    error: &CodexErr,
    current: &str,
    candidates: &'a [FallbackCandidate],
) -> Option<&'a FallbackCandidate> {
    if !is_limit_error(error) {
        return None;
    }
    candidates
        .iter()
        .filter(|candidate| candidate.slug != current)
        // Free models first: a quota limit means billing is the problem.
        .min_by_key(|candidate| !candidate.free)
}

fn is_limit_error(error: &CodexErr) -> bool {
    matches!(
        error.details(),
        CodexErrorDetails::RateLimitExceeded(_)
            | CodexErrorDetails::UsageLimitReached(_)
            | CodexErrorDetails::QuotaExceeded
            | CodexErrorDetails::APISpendLimitExceeded(_)
    )
}

#[cfg(test)]
mod tests {
    use super::FallbackCandidate;
    use super::fallback_for_error;
    use codex_protocol::error::CodexErr;
    use codex_protocol::error::CodexErrorDetails;

    fn rate_limited() -> CodexErr {
        CodexErr::new(CodexErrorDetails::RateLimitExceeded("slow down".into()))
    }

    fn candidates() -> Vec<FallbackCandidate> {
        vec![
            FallbackCandidate {
                slug: "qwen3.8-max".into(),
                free: false,
            },
            FallbackCandidate {
                slug: "qwen3.8-27b-free".into(),
                free: true,
            },
        ]
    }

    #[test]
    fn rate_limit_prefers_a_free_model() {
        let picked = fallback_for_error(&rate_limited(), "qwen3.8-max", &candidates())
            .expect("a fallback");
        assert_eq!(picked.slug, "qwen3.8-27b-free");
    }

    #[test]
    fn never_picks_the_model_that_just_failed() {
        let list = candidates();
        let picked = fallback_for_error(&rate_limited(), "qwen3.8-27b-free", &list)
            .expect("a fallback");
        assert_ne!(picked.slug, "qwen3.8-27b-free");
    }

    #[test]
    fn auth_errors_do_not_switch_models() {
        let auth = CodexErr::new(CodexErrorDetails::InvalidRequest("bad key".into()));
        assert!(fallback_for_error(&auth, "qwen3.8-max", &candidates()).is_none());
    }

    #[test]
    fn no_candidates_means_no_switch() {
        assert!(fallback_for_error(&rate_limited(), "qwen3.8-max", &[]).is_none());
    }
}
