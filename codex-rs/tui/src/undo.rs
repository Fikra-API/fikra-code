//! `/undo`: restore the files from the edit the agent made most recently.

use std::path::Path;
use std::path::PathBuf;

/// One path recorded before an edit: the bytes to write back, or `None` when
/// the file did not exist and undo should remove it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordedFile {
    pub path: PathBuf,
    pub previous_content: Option<Vec<u8>>,
}

/// The most recent patch, in patch order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordedEdit {
    pub files: Vec<RecordedFile>,
}

/// What an undo did, for the message shown to the user.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct UndoOutcome {
    pub restored: usize,
    pub removed: usize,
    pub failures: Vec<String>,
}

impl UndoOutcome {
    pub(crate) fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.restored > 0 {
            parts.push(format!("restored {}", plural(self.restored, "file")));
        }
        if self.removed > 0 {
            parts.push(format!("removed {}", plural(self.removed, "file")));
        }
        let mut text = if parts.is_empty() {
            "Nothing to undo".to_string()
        } else {
            format!("Undid the last edit: {}", parts.join(", "))
        };
        for failure in &self.failures {
            text.push('\n');
            text.push_str(failure);
        }
        text
    }
}

fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// Restores every recorded path, reporting what succeeded.
pub(crate) fn restore_edit(edit: &RecordedEdit) -> UndoOutcome {
    let mut outcome = UndoOutcome::default();
    for file in &edit.files {
        match &file.previous_content {
            Some(content) => match write_bytes(&file.path, content) {
                Ok(()) => outcome.restored += 1,
                Err(err) => outcome
                    .failures
                    .push(format!("could not restore {}: {err}", file.path.display())),
            },
            None => match remove_file(&file.path) {
                Ok(()) => outcome.removed += 1,
                Err(err) => outcome
                    .failures
                    .push(format!("could not remove {}: {err}", file.path.display())),
            },
        }
    }
    outcome
}

fn write_bytes(path: &Path, content: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
}

fn remove_file(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        // Already gone is the desired end state.
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Journal file written by the core patch handler.
const JOURNAL_FILE: &str = "edit_journal.jsonl";

/// Restores the most recent entry in the edit journal under `codex_home`.
///
/// Returns a message for the user. An empty journal is not an error: it means
/// there is nothing to undo yet.
pub(crate) fn undo_last_edit(codex_home: &Path, cwd: &PathBuf) -> std::io::Result<String> {
    let path = codex_home.join(JOURNAL_FILE);
    let body = match std::fs::read_to_string(&path) {
        Ok(body) => body,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok("Nothing to undo".to_string());
        }
        Err(err) => return Err(err),
    };
    let mut lines: Vec<&str> = body.lines().filter(|line| !line.trim().is_empty()).collect();
    let Some(last) = lines.pop() else {
        return Ok("Nothing to undo".to_string());
    };
    let Some(entry) = parse_entry(last) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "the most recent journal entry is unreadable",
        ));
    };

    // Pop before restoring: a half-applied undo must not be retried forever.
    if lines.is_empty() {
        match std::fs::remove_file(&path) {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => return Err(err),
            _ => {}
        }
    } else {
        std::fs::write(&path, format!("{}\n", lines.join("\n")))?;
    }

    let mut message = restore_edit(&entry.edit).summary();
    if entry.cwd != *cwd {
        message.push_str(&format!(
            "\n(that edit was made in {})",
            entry.cwd.display()
        ));
    }
    Ok(message)
}

struct ParsedEntry {
    cwd: PathBuf,
    edit: RecordedEdit,
}

/// Reads the small JSON shape the core handler writes. Written by hand because
/// the TUI does not depend on the core crate and the shape is fully ours.
fn parse_entry(line: &str) -> Option<ParsedEntry> {
    let cwd = PathBuf::from(unescape(json_string(line, "cwd")?));
    let files_start = line.find("\"files\":[")? + "\"files\":[".len();
    let files_end = line.rfind(']')?;
    let files_text = &line[files_start..files_end];
    let mut files = Vec::new();
    for piece in files_text.split('{').skip(1) {
        let piece = piece.split('}').next()?;
        let object = format!("{{{piece}}}");
        let path = PathBuf::from(unescape(json_string(&object, "path")?));
        let previous_content = if object.contains("\"previous\":null") {
            None
        } else {
            let encoded = json_string(&object, "previous")?;
            Some(
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
                    .ok()?,
            )
        };
        files.push(RecordedFile {
            path,
            previous_content,
        });
    }
    (!files.is_empty()).then_some(ParsedEntry {
        cwd,
        edit: RecordedEdit { files },
    })
}

fn json_string<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\":\"");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let mut end = 0;
    let mut escaped = false;
    for (index, ch) in rest.char_indices() {
        if escaped {
            escaped = false;
            end = index + ch.len_utf8();
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => break,
            _ => end = index + ch.len_utf8(),
        }
    }
    Some(&rest[..end])
}

fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::RecordedEdit;
    use super::RecordedFile;
    use super::restore_edit;

    #[test]
    fn restores_previous_bytes_and_removes_created_files() {
        let dir = tempfile::tempdir().expect("temp dir");
        let existing = dir.path().join("existing.txt");
        std::fs::write(&existing, b"after\n").expect("write current");
        let created = dir.path().join("created.txt");
        std::fs::write(&created, b"new file\n").expect("write created");

        let edit = RecordedEdit {
            files: vec![
                RecordedFile {
                    path: existing.clone(),
                    previous_content: Some(b"before\n".to_vec()),
                },
                RecordedFile {
                    path: created.clone(),
                    previous_content: None,
                },
            ],
        };

        let outcome = restore_edit(&edit);
        assert_eq!(outcome.restored, 1);
        assert_eq!(outcome.removed, 1);
        assert!(outcome.failures.is_empty());
        assert_eq!(std::fs::read(&existing).expect("read"), b"before\n");
        assert!(!created.exists());
    }

    #[test]
    fn undo_restores_from_the_journal_and_shrinks_it() {
        let home = tempfile::tempdir().expect("home");
        let work = tempfile::tempdir().expect("work");
        let file = work.path().join("edited.rs");
        std::fs::write(&file, b"after\n").expect("write current");

        // Same encoding the core handler writes.
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            b"before\n",
        );
        let line = format!(
            "{{\"cwd\":\"{}\",\"files\":[{{\"path\":\"{}\",\"previous\":\"{encoded}\"}}]}}",
            work.path().display(),
            file.display()
        );
        let journal = home.path().join("edit_journal.jsonl");
        std::fs::write(&journal, format!("{line}\n")).expect("write journal");

        let message = super::undo_last_edit(home.path(), &work.path().to_path_buf())
            .expect("undo");
        assert!(message.contains("restored 1 file"), "{message}");
        assert_eq!(std::fs::read(&file).expect("read"), b"before\n");
        assert!(!journal.exists(), "journal should be empty after the last undo");
    }

    #[test]
    fn summary_names_counts() {
        let edit = RecordedEdit { files: Vec::new() };
        let outcome = restore_edit(&edit);
        assert_eq!(outcome.summary(), "Nothing to undo");
    }
}
