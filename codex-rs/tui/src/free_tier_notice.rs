//! One-time notice that prompts on a free model are stored and used for training.

/// Marker written into `FIKRA_HOME` so the notice is shown once per install.
pub(crate) const NOTICE_MARKER: &str = "free_tier_notice_shown";

/// Models that train on prompts. Matched by suffix so a dated build still counts.
pub(crate) fn trains_on_prompts(model: &str) -> bool {
    model.ends_with("-free") || model == "qwen3.8-27b-free"
}

/// The notice text, or `None` when it should not be shown.
pub(crate) fn notice_for(model: &str, already_shown: bool) -> Option<String> {
    if already_shown || !trains_on_prompts(model) {
        return None;
    }
    Some(format!(
        "You are on {model}, a free model. Prompts sent to free models are stored \
         and used to train models. Switch with /model to a paid model plus ZDR \
         (FIKRA_ZDR=true) for private code."
    ))
}

#[cfg(test)]
mod tests {
    use super::notice_for;
    use super::trains_on_prompts;

    #[test]
    fn free_models_train_paid_models_do_not() {
        assert!(trains_on_prompts("qwen3.8-27b-free"));
        assert!(trains_on_prompts("something-free"));
        assert!(!trains_on_prompts("qwen3.8-max"));
        assert!(!trains_on_prompts("deepseek-v4.1-flash"));
    }

    #[test]
    fn shows_once_for_free_models_only() {
        let model = "qwen3.8-27b-free";
        assert!(notice_for(model, false).is_some());
        assert!(notice_for(model, true).is_none());
        assert!(notice_for("qwen3.8-max", false).is_none());
    }
}
