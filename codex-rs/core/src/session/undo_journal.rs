//! Durable record of edits, so `/undo` survives the in-memory session.
//!
//! Each applied patch appends one JSON line describing the files it touched and
//! the bytes that were there before. `/undo` pops the last line, restores those
//! files and rewrites the journal without it.

use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

/// How many edits stay undoable across the whole home directory.
pub(crate) const MAX_JOURNAL_ENTRIES: usize = 32;

/// File name under `FIKRA_HOME`.
pub(crate) const JOURNAL_FILE: &str = "edit_journal.jsonl";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JournalFile {
    pub path: PathBuf,
    /// Base64 of the previous bytes, or `None` when the file did not exist.
    pub previous: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JournalEntry {
    pub cwd: PathBuf,
    pub files: Vec<JournalFile>,
}

impl JournalFile {
    pub(crate) fn from_bytes(path: PathBuf, previous: Option<&[u8]>) -> Self {
        Self {
            path,
            previous: previous.map(|bytes| BASE64.encode(bytes)),
        }
    }
}

fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Serializes one entry as a single JSON line without pulling in a JSON crate
/// for a shape this small and fully under our control.
pub(crate) fn encode(entry: &JournalEntry) -> String {
    let files = entry
        .files
        .iter()
        .map(|file| {
            let previous = match &file.previous {
                Some(value) => format!("\"{}\"", escape(value)),
                None => "null".to_string(),
            };
            format!(
                "{{\"path\":\"{}\",\"previous\":{previous}}}",
                escape(&file.path.display().to_string())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"cwd\":\"{}\",\"files\":[{files}]}}",
        escape(&entry.cwd.display().to_string())
    )
}

fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\":\"");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    // Paths are escaped the same way encode() wrote them; unescape minimally.
    let mut out_end = 0;
    let mut escaped = false;
    for (index, ch) in rest.char_indices() {
        if escaped {
            escaped = false;
            out_end = index + ch.len_utf8();
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => break,
            _ => out_end = index + ch.len_utf8(),
        }
    }
    Some(&rest[..out_end])
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

/// Parses one line written by `encode`. Returns `None` for anything unexpected
/// rather than guessing, so a corrupt journal cannot trigger a bad restore.
pub(crate) fn decode(line: &str) -> Option<JournalEntry> {
    let cwd = unescape(field(line, "cwd")?);
    let files_start = line.find("\"files\":[")? + "\"files\":[".len();
    let files_end = line.rfind(']')?;
    let files_text = &line[files_start..files_end];
    let mut files = Vec::new();
    for piece in files_text.split("{").skip(1) {
        let piece = piece.split('}').next()?;
        let entry = format!("{{{piece}}}");
        let path = unescape(field(&entry, "path")?);
        let previous = if entry.contains("\"previous\":null") {
            None
        } else {
            Some(field(&entry, "previous")?.to_string())
        };
        files.push(JournalFile {
            path: PathBuf::from(path),
            previous,
        });
    }
    Some(JournalEntry {
        cwd: PathBuf::from(cwd),
        files,
    })
}

fn journal_path(codex_home: &Path) -> PathBuf {
    codex_home.join(JOURNAL_FILE)
}

/// Appends one edit, dropping the oldest entries past the cap.
pub(crate) fn append(codex_home: &Path, entry: &JournalEntry) -> std::io::Result<()> {
    if entry.files.is_empty() {
        return Ok(());
    }
    let path = journal_path(codex_home);
    let mut entries = read_all(codex_home)?;
    entries.push(entry.clone());
    if entries.len() > MAX_JOURNAL_ENTRIES {
        let excess = entries.len() - MAX_JOURNAL_ENTRIES;
        entries.drain(0..excess);
    }
    let mut body = String::new();
    for entry in &entries {
        body.push_str(&encode(entry));
        body.push('\n');
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Rewrite rather than append so the cap is enforced in one place.
    let mut file = std::fs::File::create(&path)?;
    file.write_all(body.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

pub(crate) fn read_all(codex_home: &Path) -> std::io::Result<Vec<JournalEntry>> {
    let path = journal_path(codex_home);
    let body = match std::fs::read_to_string(&path) {
        Ok(body) => body,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    Ok(body.lines().filter_map(decode).collect())
}

/// Removes and returns the newest entry, leaving the rest on disk.
pub(crate) fn pop(codex_home: &Path) -> std::io::Result<Option<JournalEntry>> {
    let mut entries = read_all(codex_home)?;
    let Some(entry) = entries.pop() else {
        return Ok(None);
    };
    let path = journal_path(codex_home);
    if entries.is_empty() {
        match std::fs::remove_file(&path) {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => return Err(err),
            _ => {}
        }
        return Ok(Some(entry));
    }
    let mut body = String::new();
    for entry in &entries {
        body.push_str(&encode(entry));
        body.push('\n');
    }
    std::fs::write(&path, body)?;
    Ok(Some(entry))
}

#[cfg(test)]
mod tests {
    use super::JournalEntry;
    use super::JournalFile;
    use super::append;
    use super::decode;
    use super::encode;
    use super::pop;
    use super::read_all;

    fn entry(path: &str, previous: Option<&[u8]>) -> JournalEntry {
        JournalEntry {
            cwd: std::path::PathBuf::from("/work"),
            files: vec![JournalFile::from_bytes(
                std::path::PathBuf::from(path),
                previous,
            )],
        }
    }

    #[test]
    fn round_trips_through_one_line() {
        let original = entry("/work/a.rs", Some(b"fn main() {}\n"));
        let line = encode(&original);
        assert!(!line.contains('\n'));
        assert_eq!(decode(&line), Some(original));
    }

    #[test]
    fn round_trips_a_created_file() {
        let original = entry("/work/new.rs", None);
        assert_eq!(decode(&encode(&original)), Some(original));
    }

    #[test]
    fn append_and_pop_keep_order_and_shrink_the_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        append(dir.path(), &entry("/work/one.rs", Some(b"1"))).expect("append one");
        append(dir.path(), &entry("/work/two.rs", Some(b"2"))).expect("append two");
        assert_eq!(read_all(dir.path()).expect("read").len(), 2);

        let popped = pop(dir.path()).expect("pop").expect("an entry");
        assert_eq!(popped.files[0].path, std::path::PathBuf::from("/work/two.rs"));
        let remaining = read_all(dir.path()).expect("read");
        assert_eq!(remaining.len(), 1);
        assert_eq!(
            remaining[0].files[0].path,
            std::path::PathBuf::from("/work/one.rs")
        );
    }

    #[test]
    fn pop_on_empty_home_reports_nothing_to_undo() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert_eq!(pop(dir.path()).expect("pop"), None);
    }
}
