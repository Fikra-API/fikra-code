//! Bounded file snapshots taken before an `apply_patch`, backing `/undo`.
//!
//! Only the paths a patch touches are captured, and only the bytes that were on
//! disk at that moment. Entries are kept newest-first and the stack is capped so
//! a long session cannot grow without limit.

use std::collections::VecDeque;
use std::path::PathBuf;

/// How many patches remain undoable. Sixteen is enough to walk back a bad
/// sequence without pinning much memory for large files.
pub(crate) const MAX_UNDO_ENTRIES: usize = 16;

/// The state of one path before a patch ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileSnapshot {
    pub path: PathBuf,
    /// `None` means the file did not exist, so undo deletes it.
    pub previous_content: Option<Vec<u8>>,
}

/// One patch worth of snapshots, in patch order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EditSnapshot {
    pub files: Vec<FileSnapshot>,
    /// Human label for the message shown after an undo, e.g. the patch header.
    pub call_id: String,
}

/// Newest-first stack of undoable edits.
#[derive(Debug, Default)]
pub(crate) struct EditSnapshots {
    entries: VecDeque<EditSnapshot>,
}

impl EditSnapshots {
    /// Records one applied patch, dropping the oldest entry past the cap.
    pub(crate) fn push(&mut self, snapshot: EditSnapshot) {
        if snapshot.files.is_empty() {
            return;
        }
        self.entries.push_front(snapshot);
        self.entries.truncate(MAX_UNDO_ENTRIES);
    }

    /// Removes and returns the most recent entry.
    pub(crate) fn pop(&mut self) -> Option<EditSnapshot> {
        self.entries.pop_front()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}
