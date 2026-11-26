//! Fikra: decide whether a turn that ended without a tool call actually finished.
//!
//! Two signals, in order:
//!
//! 1. `decide` — the structural signal. Today it reads the assistant message
//!    phase that the provider stream carries (`Commentary`/`PartialAnswer` mean
//!    "not done"). When every provider in the Fikra catalog sends phases, this
//!    is the whole answer and the wording of the reply stops mattering.
//! 2. [`super::unfinished_reply::looks_unfinished_after_work`] — the v0.3.3
//!    wording heuristic, kept as the fallback for providers that send no phase.
//!
//! Keep both: the heuristic covers today's models, the structural signal is
//! what we migrate onto.

use codex_protocol::models::MessagePhase;

use super::unfinished_reply;

pub(crate) enum TurnDecision {
    Finished,
    Continue,
    ContinueBecauseHeuristic,
}

impl TurnDecision {
    pub(crate) fn continues(&self) -> bool {
        !matches!(self, Self::Finished)
    }

    /// True only when the wording heuristic made the call, i.e. the provider
    /// declared nothing. Used for logging so fallback use is visible.
    pub(crate) fn is_heuristic_continue(&self) -> bool {
        matches!(self, Self::ContinueBecauseHeuristic)
    }
}

/// Structural signal only: what the provider declared about this message.
pub(crate) fn finished_by_structure(phase: Option<&MessagePhase>) -> Option<bool> {
    match phase {
        Some(MessagePhase::FinalAnswer) => Some(true),
        Some(MessagePhase::Commentary | MessagePhase::PartialAnswer) => Some(false),
        // No declaration: the structural signal says nothing either way.
        None => None,
    }
}

/// Combine the two signals for a turn that ended without a tool call.
pub(crate) fn decide(
    phase: Option<&MessagePhase>,
    reply: Option<&str>,
    worked_this_turn: bool,
) -> TurnDecision {
    match finished_by_structure(phase) {
        Some(true) => TurnDecision::Finished,
        Some(false) => TurnDecision::Continue,
        None => {
            if unfinished_reply::looks_unfinished_after_work(reply, worked_this_turn) {
                TurnDecision::ContinueBecauseHeuristic
            } else {
                TurnDecision::Finished
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TurnDecision;
    use super::decide;
    use super::finished_by_structure;
    use codex_protocol::models::MessagePhase;

    #[test]
    fn structural_signal_reads_the_phase() {
        assert_eq!(
            finished_by_structure(Some(&MessagePhase::FinalAnswer)),
            Some(true)
        );
        assert_eq!(
            finished_by_structure(Some(&MessagePhase::Commentary)),
            Some(false)
        );
        assert_eq!(
            finished_by_structure(Some(&MessagePhase::PartialAnswer)),
            Some(false)
        );
        assert_eq!(finished_by_structure(None), None);
    }

    #[test]
    fn declared_final_answer_wins_over_wording() {
        // The heuristic alone would continue this; the phase says it is done.
        let reply = "Now I'll update the README.";
        assert!(matches!(
            decide(Some(&MessagePhase::FinalAnswer), Some(reply), true),
            TurnDecision::Finished
        ));
    }

    #[test]
    fn declared_commentary_continues_even_when_wording_looks_finished() {
        let reply = "Done. Added pagination and all 14 tests pass.";
        assert!(matches!(
            decide(Some(&MessagePhase::Commentary), Some(reply), true),
            TurnDecision::Continue
        ));
    }

    #[test]
    fn missing_phase_falls_back_to_the_v033_heuristic() {
        assert!(matches!(
            decide(None, Some("21/21. Let me run the whole suite."), true),
            TurnDecision::ContinueBecauseHeuristic
        ));
        assert!(matches!(
            decide(None, Some("Fixed the bug; all 14 tests pass."), true),
            TurnDecision::Finished
        ));
        assert!(matches!(
            decide(None, None, false),
            TurnDecision::ContinueBecauseHeuristic
        ));
    }
}
