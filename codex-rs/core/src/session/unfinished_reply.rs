//! Fikra: keep a turn going when the model stops on an announcement.
//!
//! Some models (DeepSeek in particular) end a long turn with text such as
//! "21/21. Let me run the whole suite..." and no tool call. The loop treats
//! any reply without a tool call as final, so the turn ends on a half step
//! and the user never gets a summary. When the final reply is empty or its
//! last sentence announces more work, the turn is continued with a short
//! prompt, at most [`MAX_AUTO_CONTINUES`] times per turn (long runs can stall several times).

/// Nudges in a row without any tool work in between.
pub(crate) const MAX_AUTO_CONTINUES: u8 = 6;
/// Absolute ceiling per turn, even when the model keeps making progress.
pub(crate) const MAX_AUTO_CONTINUES_PER_TURN: u8 = 40;

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

/// Words ending in "-ing" that are not progressive verbs.
const NOT_GERUNDS: &[&str] = &[
    "nothing", "something", "anything", "everything", "thing", "things", "string", "strings",
    "during", "morning", "evening", "king", "ring", "sing", "spring", "bring", "wing", "ceiling",
    "building", "meaning", "setting", "settings", "warning", "warnings", "pending", "missing",
    "remaining", "existing", "following", "interesting", "outstanding", "according", "padding",
];

fn is_gerund(word: &str) -> bool {
    word.len() >= 5 && word.ends_with("ing") && !NOT_GERUNDS.contains(&word)
}

const DONE_WORDS: &[&str] = &[
    "done", "fixed", "complete", "completed", "finished", "passes", "passed", "pass", "sent",
    "delivered", "ready", "merged", "pushed", "deployed", "works", "working now", "all set",
    "summary", "nothing changed", "no changes", "resolved", "saved", "created", "updated",
];

/// Stronger check once the turn has already run tools: a short final reply
/// that is not a question and does not report a result means the model
/// narrated its next step and stopped ("Probing the real contract.").
pub(crate) fn looks_unfinished_after_work(reply: Option<&str>, worked_this_turn: bool) -> bool {
    if looks_unfinished(reply) {
        return true;
    }
    if !worked_this_turn {
        return false;
    }
    let text = reply.unwrap_or("").trim();
    if text.chars().count() > 300 || text.contains('?') {
        return false;
    }
    !reports_result(&text.to_lowercase())
}

fn reports_result(lower: &str) -> bool {
    DONE_WORDS.iter().any(|w| lower.contains(w))
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
    // "Now an end-to-end check through the MCP handler." announces the next
    // step; "Now all tests pass." reports a result.
    if matches!(first_word, "now" | "next") && !reports_result(&last) {
        return true;
    }
    if matches!(first_word, "now" | "next" | "then" | "so" | "also" | "still" | "just") {
        first_word = words.next().unwrap_or("");
    }
    if GERUND_STARTS.contains(&first_word) || is_gerund(first_word) {
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
        assert!(looks_unfinished(Some("Rendering both emails with the existing brand shell and checking the output isn't blank.")));
        assert!(looks_unfinished(Some("The floor is wrong for a legitimate short reply. Lowering it.")));
        assert!(looks_unfinished(Some("It responds (405 on GET), it's OpenAI-compatible TTS. Probing the real contract.")));
        assert!(looks_unfinished(Some("Now doing a real synthesis so I can report formats and the exact parameters.")));
        assert!(looks_unfinished(Some("Tests added. Also checking the checkout API on already-active plans.")));
        assert!(looks_unfinished(Some("All 32 pass. Now an end-to-end check through the actual MCP handler with the in-memory store, so the tool wiring is proven too.")));
        assert!(looks_unfinished(Some("Schema is in. Next the migration for the nexus tables.")));
        assert!(!looks_unfinished(Some("Fixed the race. Now all 32 tests pass.")));
    }

    #[test]
    fn after_work_short_narration_continues() {
        use super::looks_unfinished_after_work as f;
        assert!(f(Some("The schema lives in db.rs. The migration needs a new column."), true));
        assert!(!f(Some("The schema lives in db.rs. The migration needs a new column."), false));
        assert!(!f(Some("Fixed: the migration adds the column and all 12 tests pass."), true));
        assert!(!f(Some("Which database should I use, Postgres or SQLite?"), true));
        assert!(!f(Some("Hi! What are we working on today?"), false));
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
