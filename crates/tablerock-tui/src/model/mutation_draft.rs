//! In-memory staged mutation drafts for one grid tab.
//!
//! Staging never touches the database. Apply builds a typed plan elsewhere
//! from these drafts; preview text is never re-parsed for execution.

use tablerock_core::{EditabilityFacts, EditabilityReason};

/// Row/cell marker for pending state (text+glyph; never color alone).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DraftMarker {
    #[default]
    Unchanged,
    Inserted,
    Modified,
    Deleted,
}

impl DraftMarker {
    #[must_use]
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Unchanged => "",
            Self::Inserted => "+",
            Self::Modified => "·",
            Self::Deleted => "−",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Inserted => "inserted",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
        }
    }
}

/// Locator field (identity column → original display text for plan building).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftLocatorField {
    pub column: String,
    pub original_text: String,
}

/// One staged cell update on an existing row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedCellEdit {
    pub abs_row: u64,
    pub column: String,
    pub original_text: String,
    pub staged_text: String,
    pub locator: Vec<DraftLocatorField>,
}

/// One staged insert row (column → staged text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedInsert {
    pub draft_id: u64,
    pub values: Vec<(String, String)>,
}

/// One staged delete of an existing row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedDelete {
    pub abs_row: u64,
    pub locator: Vec<DraftLocatorField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum UndoEntry {
    /// Prior cell edit replaced by a newer one (for undo restore).
    CellReplaced {
        previous: Option<StagedCellEdit>,
        current: StagedCellEdit,
    },
    Insert(StagedInsert),
    Delete(StagedDelete),
}

/// Per-tab staged mutation set.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MutationDraftModel {
    pub cell_edits: Vec<StagedCellEdit>,
    pub inserts: Vec<StagedInsert>,
    pub deletes: Vec<StagedDelete>,
    next_insert_id: u64,
    undo: Vec<UndoEntry>,
    /// When false, stage methods are no-ops (ReadOnly profile / non-editable).
    staging_allowed: bool,
    block_reason: Option<EditabilityReason>,
}

impl MutationDraftModel {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sync from editability facts; ReadOnly clears staging affordance.
    pub fn apply_editability(&mut self, facts: &EditabilityFacts) {
        match facts {
            EditabilityFacts::Editable { .. } => {
                self.staging_allowed = true;
                self.block_reason = None;
            }
            EditabilityFacts::ReadOnly { reason } => {
                self.staging_allowed = false;
                self.block_reason = Some(*reason);
                // Fail closed: never keep drafts when no longer editable.
                self.discard_all();
            }
        }
    }

    #[must_use]
    pub const fn staging_allowed(&self) -> bool {
        self.staging_allowed
    }

    #[must_use]
    pub const fn block_reason(&self) -> Option<EditabilityReason> {
        self.block_reason
    }

    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.cell_edits.len() + self.inserts.len() + self.deletes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending_count() == 0
    }

    /// Stage a cell change. Returns false if staging is blocked or value equal.
    pub fn stage_cell_edit(&mut self, edit: StagedCellEdit) -> bool {
        if !self.staging_allowed {
            return false;
        }
        if edit.original_text == edit.staged_text {
            return false;
        }
        if self.deletes.iter().any(|d| d.abs_row == edit.abs_row) {
            return false;
        }
        let previous = self
            .cell_edits
            .iter()
            .position(|e| e.abs_row == edit.abs_row && e.column == edit.column)
            .map(|idx| self.cell_edits.remove(idx));
        self.undo.push(UndoEntry::CellReplaced {
            previous,
            current: edit.clone(),
        });
        self.cell_edits.push(edit);
        true
    }

    pub fn stage_insert(&mut self, values: Vec<(String, String)>) -> Option<u64> {
        if !self.staging_allowed {
            return None;
        }
        let draft_id = self.next_insert_id;
        self.next_insert_id = self.next_insert_id.saturating_add(1);
        let insert = StagedInsert { draft_id, values };
        self.undo.push(UndoEntry::Insert(insert.clone()));
        self.inserts.push(insert);
        Some(draft_id)
    }

    /// Replace values on an existing insert draft. Returns false if missing/blocked.
    pub fn replace_insert_values(&mut self, draft_id: u64, values: Vec<(String, String)>) -> bool {
        if !self.staging_allowed {
            return false;
        }
        let Some(insert) = self.inserts.iter_mut().find(|i| i.draft_id == draft_id) else {
            return false;
        };
        insert.values = values;
        true
    }

    /// Most recently staged insert (last in list), if any.
    #[must_use]
    pub fn last_insert(&self) -> Option<&StagedInsert> {
        self.inserts.last()
    }

    /// Discard only the last staged insert. Returns true if one was removed.
    pub fn discard_last_insert(&mut self) -> bool {
        if !self.staging_allowed || self.inserts.is_empty() {
            return false;
        }
        let removed = self.inserts.pop();
        // Drop matching undo entry if it is the tip Insert for this draft.
        if let Some(ins) = removed.as_ref() {
            if let Some(UndoEntry::Insert(top)) = self.undo.last() {
                if top.draft_id == ins.draft_id {
                    self.undo.pop();
                }
            }
        }
        removed.is_some()
    }

    /// Discard a staged cell edit for one absolute row + column.
    pub fn discard_cell_edit(&mut self, abs_row: u64, column: &str) -> bool {
        let before = self.cell_edits.len();
        self.cell_edits
            .retain(|e| !(e.abs_row == abs_row && e.column == column));
        let removed = self.cell_edits.len() < before;
        if removed {
            // Drop undo tips that only re-apply this cell (best-effort).
            self.undo.retain(|entry| match entry {
                UndoEntry::CellReplaced { current, .. } => {
                    !(current.abs_row == abs_row && current.column == column)
                }
                _ => true,
            });
        }
        removed
    }

    /// Discard a staged delete for one absolute row.
    pub fn discard_delete(&mut self, abs_row: u64) -> bool {
        let before = self.deletes.len();
        self.deletes.retain(|d| d.abs_row != abs_row);
        let removed = self.deletes.len() < before;
        if removed {
            self.undo.retain(|entry| match entry {
                UndoEntry::Delete(d) => d.abs_row != abs_row,
                _ => true,
            });
        }
        removed
    }

    /// Discard all cell edits and delete stages for one absolute row.
    pub fn discard_row_stages(&mut self, abs_row: u64) -> bool {
        let cell = {
            let before = self.cell_edits.len();
            self.cell_edits.retain(|e| e.abs_row != abs_row);
            self.cell_edits.len() < before
        };
        let del = self.discard_delete(abs_row);
        if cell {
            self.undo.retain(|entry| match entry {
                UndoEntry::CellReplaced { current, .. } => current.abs_row != abs_row,
                _ => true,
            });
        }
        cell || del
    }

    pub fn stage_delete(&mut self, delete: StagedDelete) -> bool {
        if !self.staging_allowed {
            return false;
        }
        // Drop pending cell edits on the same row.
        self.cell_edits.retain(|e| e.abs_row != delete.abs_row);
        if self.deletes.iter().any(|d| d.abs_row == delete.abs_row) {
            return false;
        }
        self.undo.push(UndoEntry::Delete(delete.clone()));
        self.deletes.push(delete);
        true
    }

    pub fn discard_all(&mut self) {
        self.cell_edits.clear();
        self.inserts.clear();
        self.deletes.clear();
        self.undo.clear();
    }

    /// Undo last stage action.
    pub fn undo(&mut self) -> bool {
        let Some(entry) = self.undo.pop() else {
            return false;
        };
        match entry {
            UndoEntry::CellReplaced {
                previous: None,
                current: edit,
            } => {
                self.cell_edits
                    .retain(|e| !(e.abs_row == edit.abs_row && e.column == edit.column));
            }
            UndoEntry::CellReplaced {
                previous: Some(prev),
                current,
            } => {
                self.cell_edits
                    .retain(|e| !(e.abs_row == current.abs_row && e.column == current.column));
                self.cell_edits.push(prev);
            }
            UndoEntry::Insert(insert) => {
                self.inserts.retain(|i| i.draft_id != insert.draft_id);
            }
            UndoEntry::Delete(delete) => {
                self.deletes.retain(|d| d.abs_row != delete.abs_row);
            }
        }
        true
    }

    /// Marker for an absolute data row (inserts are not absolute rows).
    #[must_use]
    pub fn row_marker(&self, abs_row: u64) -> DraftMarker {
        if self.deletes.iter().any(|d| d.abs_row == abs_row) {
            return DraftMarker::Deleted;
        }
        if self.cell_edits.iter().any(|e| e.abs_row == abs_row) {
            return DraftMarker::Modified;
        }
        DraftMarker::Unchanged
    }

    /// Marker for one cell (modified if staged).
    #[must_use]
    pub fn cell_marker(&self, abs_row: u64, column: &str) -> DraftMarker {
        if self.deletes.iter().any(|d| d.abs_row == abs_row) {
            return DraftMarker::Deleted;
        }
        if self
            .cell_edits
            .iter()
            .any(|e| e.abs_row == abs_row && e.column == column)
        {
            return DraftMarker::Modified;
        }
        DraftMarker::Unchanged
    }

    /// Original value if cell is modified (for "reachable original").
    #[must_use]
    pub fn original_for_cell(&self, abs_row: u64, column: &str) -> Option<&str> {
        self.cell_edits
            .iter()
            .find(|e| e.abs_row == abs_row && e.column == column)
            .map(|e| e.original_text.as_str())
    }

    /// Staged value overlay for display.
    #[must_use]
    pub fn staged_for_cell(&self, abs_row: u64, column: &str) -> Option<&str> {
        self.cell_edits
            .iter()
            .find(|e| e.abs_row == abs_row && e.column == column)
            .map(|e| e.staged_text.as_str())
    }

    #[must_use]
    pub fn status_suffix(&self) -> String {
        let n = self.pending_count();
        if n == 0 {
            return String::new();
        }
        format!(
            " · staged {n} ({}↑ {}· {}↓)",
            self.inserts.len(),
            self.cell_edits.len(),
            self.deletes.len()
        )
    }

    /// Multi-line inventory of staged drafts for the inspector panel.
    #[must_use]
    pub fn staged_panel_text(&self) -> String {
        if self.is_empty() {
            return "no staged changes".into();
        }
        let mut lines = vec![format!(
            "{} staged ({} insert, {} cell, {} delete)",
            self.pending_count(),
            self.inserts.len(),
            self.cell_edits.len(),
            self.deletes.len()
        )];
        for ins in &self.inserts {
            let cols: Vec<String> = ins
                .values
                .iter()
                .map(|(c, v)| {
                    if v.is_empty() {
                        format!("{c}=∅")
                    } else {
                        format!("{c}={v}")
                    }
                })
                .take(8)
                .collect();
            let more = if ins.values.len() > 8 {
                format!(" +{} more", ins.values.len() - 8)
            } else {
                String::new()
            };
            lines.push(format!(
                "+ insert #{}: {}{more}",
                ins.draft_id,
                cols.join(", ")
            ));
        }
        for edit in &self.cell_edits {
            lines.push(format!(
                "· r{} {} : {:?} → {:?}",
                edit.abs_row, edit.column, edit.original_text, edit.staged_text
            ));
        }
        for del in &self.deletes {
            let keys: Vec<String> = del
                .locator
                .iter()
                .map(|f| format!("{}={}", f.column, f.original_text))
                .collect();
            lines.push(format!("− r{} delete ({})", del.abs_row, keys.join(", ")));
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tablerock_core::ProfileSafetyMode;

    fn editable() -> EditabilityFacts {
        EditabilityFacts::classify(
            ProfileSafetyMode::ConfirmWrites,
            false,
            Some("public"),
            Some("users"),
            &["id".into()],
        )
    }

    fn read_only() -> EditabilityFacts {
        EditabilityFacts::classify(
            ProfileSafetyMode::ReadOnly,
            false,
            Some("public"),
            Some("users"),
            &["id".into()],
        )
    }

    fn cell(row: u64, col: &str, from: &str, to: &str) -> StagedCellEdit {
        StagedCellEdit {
            abs_row: row,
            column: col.into(),
            original_text: from.into(),
            staged_text: to.into(),
            locator: vec![DraftLocatorField {
                column: "id".into(),
                original_text: row.to_string(),
            }],
        }
    }

    #[test]
    fn read_only_profile_cannot_stage() {
        let mut draft = MutationDraftModel::new();
        draft.apply_editability(&read_only());
        assert!(!draft.staging_allowed());
        assert!(!draft.stage_cell_edit(cell(0, "name", "a", "b")));
        assert!(
            draft
                .stage_insert(vec![("id".into(), "1".into())])
                .is_none()
        );
        assert!(!draft.stage_delete(StagedDelete {
            abs_row: 0,
            locator: vec![],
        }));
        assert!(draft.is_empty());
    }

    #[test]
    fn stage_undo_and_discard() {
        let mut draft = MutationDraftModel::new();
        draft.apply_editability(&editable());
        assert!(draft.stage_cell_edit(cell(1, "name", "alice", "bob")));
        assert_eq!(draft.pending_count(), 1);
        assert_eq!(draft.row_marker(1), DraftMarker::Modified);
        assert_eq!(draft.original_for_cell(1, "name"), Some("alice"));
        assert_eq!(draft.staged_for_cell(1, "name"), Some("bob"));
        assert!(draft.undo());
        assert!(draft.is_empty());
        assert!(draft.stage_cell_edit(cell(1, "name", "alice", "bob")));
        draft.discard_all();
        assert!(draft.is_empty());
    }

    #[test]
    fn delete_clears_cell_edits_on_same_row() {
        let mut draft = MutationDraftModel::new();
        draft.apply_editability(&editable());
        assert!(draft.stage_cell_edit(cell(2, "name", "x", "y")));
        assert!(draft.stage_delete(StagedDelete {
            abs_row: 2,
            locator: vec![DraftLocatorField {
                column: "id".into(),
                original_text: "2".into(),
            }],
        }));
        assert_eq!(draft.cell_edits.len(), 0);
        assert_eq!(draft.row_marker(2), DraftMarker::Deleted);
        assert_eq!(draft.pending_count(), 1);
    }

    #[test]
    fn switching_to_read_only_discards_staged() {
        let mut draft = MutationDraftModel::new();
        draft.apply_editability(&editable());
        assert!(draft.stage_cell_edit(cell(0, "n", "a", "b")));
        draft.apply_editability(&read_only());
        assert!(draft.is_empty());
        assert!(!draft.staging_allowed());
    }

    #[test]
    fn insert_counts_and_undo() {
        let mut draft = MutationDraftModel::new();
        draft.apply_editability(&editable());
        let id = draft
            .stage_insert(vec![("id".into(), "9".into()), ("name".into(), "z".into())])
            .unwrap();
        assert_eq!(id, 0);
        assert_eq!(draft.inserts.len(), 1);
        assert!(draft.undo());
        assert!(draft.inserts.is_empty());
    }

    #[test]
    fn replace_insert_values_updates_last() {
        let mut draft = MutationDraftModel::new();
        draft.apply_editability(&editable());
        let id = draft
            .stage_insert(vec![
                ("id".into(), String::new()),
                ("name".into(), String::new()),
            ])
            .unwrap();
        assert!(draft.replace_insert_values(
            id,
            vec![("id".into(), "1".into()), ("name".into(), "ada".into())]
        ));
        let last = draft.last_insert().unwrap();
        assert_eq!(last.values[0].1, "1");
        assert_eq!(last.values[1].1, "ada");
        assert!(!draft.replace_insert_values(99, vec![("x".into(), "y".into())]));
    }

    #[test]
    fn discard_cell_and_row_stages() {
        let mut draft = MutationDraftModel::new();
        draft.apply_editability(&editable());
        assert!(draft.stage_cell_edit(cell(1, "name", "a", "b")));
        assert!(draft.stage_cell_edit(cell(1, "age", "1", "2")));
        assert!(draft.stage_cell_edit(cell(2, "name", "x", "y")));
        assert!(draft.discard_cell_edit(1, "name"));
        assert!(
            !draft
                .cell_edits
                .iter()
                .any(|e| e.column == "name" && e.abs_row == 1)
        );
        assert!(
            draft
                .cell_edits
                .iter()
                .any(|e| e.column == "age" && e.abs_row == 1)
        );
        assert!(draft.stage_delete(StagedDelete {
            abs_row: 2,
            locator: vec![DraftLocatorField {
                column: "id".into(),
                original_text: "2".into(),
            }],
        }));
        // Delete drops pending cell edits on row 2.
        assert!(!draft.cell_edits.iter().any(|e| e.abs_row == 2));
        assert!(draft.discard_row_stages(2));
        assert!(!draft.deletes.iter().any(|d| d.abs_row == 2));
        assert!(draft.discard_row_stages(1));
        assert!(draft.cell_edits.is_empty());
        assert!(!draft.discard_cell_edit(9, "nope"));
    }

    #[test]
    fn discard_last_insert_only() {
        let mut draft = MutationDraftModel::new();
        draft.apply_editability(&editable());
        draft.stage_insert(vec![("id".into(), "1".into())]).unwrap();
        draft.stage_insert(vec![("id".into(), "2".into())]).unwrap();
        assert!(draft.stage_cell_edit(cell(0, "name", "a", "b")));
        assert!(draft.discard_last_insert());
        assert_eq!(draft.inserts.len(), 1);
        assert_eq!(draft.inserts[0].values[0].1, "1");
        assert_eq!(draft.cell_edits.len(), 1);
        assert!(draft.discard_last_insert());
        assert!(draft.inserts.is_empty());
        assert!(!draft.discard_last_insert());
        assert_eq!(draft.cell_edits.len(), 1);
    }

    #[test]
    fn staged_panel_text_lists_all_kinds() {
        let mut draft = MutationDraftModel::new();
        draft.apply_editability(&editable());
        assert_eq!(draft.staged_panel_text(), "no staged changes");
        draft.stage_insert(vec![("id".into(), "1".into())]).unwrap();
        assert!(draft.stage_cell_edit(cell(0, "name", "a", "b")));
        assert!(draft.stage_delete(StagedDelete {
            abs_row: 3,
            locator: vec![DraftLocatorField {
                column: "id".into(),
                original_text: "3".into(),
            }],
        }));
        let panel = draft.staged_panel_text();
        assert!(panel.contains("insert"), "{panel}");
        assert!(panel.contains("name"), "{panel}");
        assert!(panel.contains("delete"), "{panel}");
    }
}
