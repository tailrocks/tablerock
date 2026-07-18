//! Presentation-local multiline SQL editor model (TermRock TextArea backed in view).
//!
//! Statement spans are recomputed outside render via core `statements` (never
//! naive semicolon splitting). Run uses selection when set, else the current
//! statement under the cursor.

use tablerock_core::{SqlDialect, StatementSpan, statement_at, statements};

/// Editor text revision (monotonic; no clocks).
pub type EditorRevision = u64;

/// Projection of a statement span for status/highlight (no core types in view).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatementSpanView {
    pub start: usize,
    pub end: usize,
    pub complete: bool,
}

impl From<StatementSpan> for StatementSpanView {
    fn from(span: StatementSpan) -> Self {
        Self {
            start: span.start,
            end: span.end,
            complete: span.complete,
        }
    }
}

/// Multiline SQL editor state for one workbench SQL tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryEditorModel {
    text: String,
    /// UTF-8 byte offset of the caret in `text`.
    cursor: usize,
    /// Inclusive-start exclusive-end selection when active.
    selection: Option<(usize, usize)>,
    /// Dialect label for boundary analysis (`PostgreSQL` / `ClickHouse`).
    dialect: SqlDialect,
    /// Monotonic text revision for completion staleness (plan 011 later).
    revision: EditorRevision,
    /// Precomputed statement spans (outside render).
    spans: Vec<StatementSpanView>,
    /// Editor/results vertical split ratio: editor height percent (20–80).
    split_editor_percent: u8,
    focused: bool,
}

impl Default for QueryEditorModel {
    fn default() -> Self {
        Self::new(SqlDialect::PostgreSql)
    }
}

impl QueryEditorModel {
    #[must_use]
    pub fn new(dialect: SqlDialect) -> Self {
        let mut model = Self {
            text: String::new(),
            cursor: 0,
            selection: None,
            dialect,
            revision: 1,
            spans: Vec::new(),
            split_editor_percent: 40,
            focused: true,
        };
        model.recompute_spans();
        model
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    #[must_use]
    pub const fn selection(&self) -> Option<(usize, usize)> {
        self.selection
    }

    #[must_use]
    pub const fn revision(&self) -> EditorRevision {
        self.revision
    }

    #[must_use]
    pub const fn dialect(&self) -> SqlDialect {
        self.dialect
    }

    #[must_use]
    pub fn spans(&self) -> &[StatementSpanView] {
        &self.spans
    }

    #[must_use]
    pub const fn split_editor_percent(&self) -> u8 {
        self.split_editor_percent
    }

    #[must_use]
    pub const fn focused(&self) -> bool {
        self.focused
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn set_dialect(&mut self, dialect: SqlDialect) {
        if self.dialect != dialect {
            self.dialect = dialect;
            self.recompute_spans();
        }
    }

    /// Remembered split: clamp to 20–80% editor height.
    pub fn set_split_editor_percent(&mut self, percent: u8) {
        self.split_editor_percent = percent.clamp(20, 80);
    }

    /// Replace entire buffer (open file / history restore).
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor = self.text.len();
        self.selection = None;
        self.bump_revision();
        self.recompute_spans();
    }

    /// Insert text at the cursor (or replace selection).
    pub fn insert(&mut self, fragment: &str) {
        if let Some((a, b)) = self.selection {
            let (start, end) = ordered(a, b);
            self.text.replace_range(start..end, fragment);
            self.cursor = start + fragment.len();
            self.selection = None;
        } else {
            let at = self.cursor.min(self.text.len());
            self.text.insert_str(at, fragment);
            self.cursor = at + fragment.len();
        }
        self.bump_revision();
        self.recompute_spans();
    }

    /// Delete the selection or the grapheme before the cursor (backspace).
    pub fn backspace(&mut self) {
        if let Some((a, b)) = self.selection {
            let (start, end) = ordered(a, b);
            self.text.replace_range(start..end, "");
            self.cursor = start;
            self.selection = None;
        } else if self.cursor > 0 {
            let prev = prev_boundary(&self.text, self.cursor);
            self.text.replace_range(prev..self.cursor, "");
            self.cursor = prev;
        } else {
            return;
        }
        self.bump_revision();
        self.recompute_spans();
    }

    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor.min(self.text.len());
        self.selection = None;
    }

    /// Set an explicit selection range (for run-selection tests / mouse later).
    pub fn set_selection(&mut self, start: usize, end: usize) {
        let start = start.min(self.text.len());
        let end = end.min(self.text.len());
        if start == end {
            self.selection = None;
            self.cursor = start;
        } else {
            self.selection = Some((start, end));
            self.cursor = end;
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Statement under the cursor, if any.
    #[must_use]
    pub fn current_statement_span(&self) -> Option<StatementSpanView> {
        statement_at(&self.text, self.dialect, self.cursor).map(StatementSpanView::from)
    }

    /// Find next occurrence of `needle` at or after cursor (literal, case-sensitive
    /// unless `case_insensitive`). Returns start byte index or None.
    #[must_use]
    pub fn find_next(&self, needle: &str, case_insensitive: bool) -> Option<usize> {
        if needle.is_empty() {
            return None;
        }
        let from = self.cursor.min(self.text.len());
        let hay = &self.text[from..];
        let rel = if case_insensitive {
            hay.to_ascii_lowercase().find(&needle.to_ascii_lowercase())
        } else {
            hay.find(needle)
        }?;
        Some(from + rel)
    }

    /// Replace the first match at/after cursor; moves cursor after replacement.
    /// Returns true when a replacement occurred.
    pub fn replace_next(
        &mut self,
        needle: &str,
        replacement: &str,
        case_insensitive: bool,
    ) -> bool {
        let Some(start) = self.find_next(needle, case_insensitive) else {
            return false;
        };
        let end = start + needle.len();
        if end > self.text.len() {
            return false;
        }
        // Case-insensitive match may differ in length of actual slice — use found length.
        let actual_end = if case_insensitive {
            // Re-find exact byte length of match in original by comparing lowered.
            let hay = &self.text[start..];
            let n = needle.len().min(hay.len());
            start + n
        } else {
            end
        };
        self.text.replace_range(start..actual_end, replacement);
        self.cursor = start + replacement.len();
        self.selection = None;
        self.bump_revision();
        self.recompute_spans();
        true
    }

    /// Replace all occurrences (left-to-right, non-overlapping). Returns count.
    pub fn replace_all(
        &mut self,
        needle: &str,
        replacement: &str,
        case_insensitive: bool,
    ) -> usize {
        if needle.is_empty() {
            return 0;
        }
        let mut count = 0usize;
        self.cursor = 0;
        while self.replace_next(needle, replacement, case_insensitive) {
            count += 1;
            if count > 10_000 {
                break; // safety bound
            }
        }
        count
    }

    /// Text Run should execute: selection if non-empty, else current statement.
    #[must_use]
    pub fn run_text(&self) -> Option<String> {
        if let Some((a, b)) = self.selection {
            let (start, end) = ordered(a, b);
            let slice = self.text[start..end].trim();
            if !slice.is_empty() {
                return Some(slice.to_owned());
            }
        }
        let span = self.current_statement_span()?;
        let slice = self.text[span.start..span.end.min(self.text.len())].trim();
        if slice.is_empty() {
            None
        } else {
            // Strip trailing semicolon for execution payload (engine accepts either).
            Some(slice.trim_end_matches(';').trim().to_owned())
        }
    }

    /// One-line status: revision, span index, complete flag.
    #[must_use]
    pub fn status_line(&self) -> String {
        let current = self.current_statement_span();
        let idx = current.and_then(|c| {
            self.spans
                .iter()
                .position(|s| s.start == c.start && s.end == c.end)
                .map(|i| i + 1)
        });
        let complete = current.map(|c| c.complete).unwrap_or(true);
        let sel = self
            .selection
            .map(|(a, b)| {
                let (s, e) = ordered(a, b);
                format!(" sel {}..{}", s, e)
            })
            .unwrap_or_default();
        format!(
            "sql rev {} · stmt {}/{}{}{}",
            self.revision,
            idx.unwrap_or(0),
            self.spans.len(),
            if complete { "" } else { " incomplete" },
            sel
        )
    }

    fn recompute_spans(&mut self) {
        self.spans = statements(&self.text, self.dialect)
            .into_iter()
            .map(StatementSpanView::from)
            .collect();
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1).max(1);
    }
}

fn ordered(a: usize, b: usize) -> (usize, usize) {
    if a <= b { (a, b) } else { (b, a) }
}

fn prev_boundary(text: &str, cursor: usize) -> usize {
    if cursor == 0 {
        return 0;
    }
    let mut i = cursor - 1;
    while i > 0 && !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_uses_selection_when_present() {
        let mut ed = QueryEditorModel::new(SqlDialect::PostgreSql);
        ed.set_text("SELECT 1; SELECT 2; SELECT 3");
        // Select middle statement including semicolon region text " SELECT 2"
        let spans = ed.spans().to_vec();
        assert!(spans.len() >= 2);
        ed.set_selection(spans[1].start, spans[1].end);
        let run = ed.run_text().expect("selection");
        assert!(run.contains('2'), "{run}");
        assert!(!run.contains('3'));
    }

    #[test]
    fn run_uses_current_statement_without_selection() {
        let mut ed = QueryEditorModel::new(SqlDialect::PostgreSql);
        ed.set_text("SELECT 1; SELECT 2");
        let spans = ed.spans().to_vec();
        assert_eq!(spans.len(), 2);
        // Cursor inside first statement.
        ed.set_cursor(spans[0].start + 2);
        let run = ed.run_text().expect("current");
        assert!(run.starts_with("SELECT 1"), "{run}");
        ed.set_cursor(spans[1].start + 2);
        let run2 = ed.run_text().expect("second");
        assert!(run2.starts_with("SELECT 2"), "{run2}");
    }

    #[test]
    fn insert_and_paste_recompute_spans_and_dirty_revision() {
        let mut ed = QueryEditorModel::new(SqlDialect::PostgreSql);
        let r0 = ed.revision();
        ed.insert("SELECT 'a;b';");
        assert!(ed.revision() > r0);
        assert_eq!(ed.spans().len(), 1);
        ed.insert("\nSELECT 2");
        assert_eq!(ed.spans().len(), 2);
        // Embedded semicolon in string is not a second statement alone.
        let mut ed2 = QueryEditorModel::new(SqlDialect::PostgreSql);
        ed2.set_text("SELECT 'x;y'");
        assert_eq!(ed2.spans().len(), 1);
    }

    #[test]
    fn incomplete_input_never_panics() {
        let mut ed = QueryEditorModel::new(SqlDialect::PostgreSql);
        ed.set_text("SELECT * FROM t WHERE name = '");
        assert!(!ed.spans().is_empty());
        assert!(ed.current_statement_span().is_some_and(|s| !s.complete));
        let _ = ed.run_text();
    }

    #[test]
    fn find_and_replace_literal() {
        let mut ed = QueryEditorModel::new(SqlDialect::PostgreSql);
        ed.set_text("SELECT foo FROM foo");
        ed.set_cursor(0);
        assert_eq!(ed.find_next("foo", false), Some(7));
        assert!(ed.replace_next("foo", "bar", false));
        assert_eq!(ed.text(), "SELECT bar FROM foo");
        assert!(ed.replace_next("foo", "bar", false));
        assert_eq!(ed.text(), "SELECT bar FROM bar");
        ed.set_text("Aa Bb aa");
        ed.set_cursor(0);
        assert_eq!(ed.replace_all("aa", "X", true), 2);
        assert_eq!(ed.text(), "X Bb X");
    }

    #[test]
    fn dollar_quote_body_is_one_statement() {
        let mut ed = QueryEditorModel::new(SqlDialect::PostgreSql);
        ed.set_text("SELECT $$a;b$$; SELECT 1");
        assert_eq!(ed.spans().len(), 2);
        assert!(ed.spans()[0].complete);
    }

    #[test]
    fn split_percent_clamped() {
        let mut ed = QueryEditorModel::default();
        ed.set_split_editor_percent(5);
        assert_eq!(ed.split_editor_percent(), 20);
        ed.set_split_editor_percent(99);
        assert_eq!(ed.split_editor_percent(), 80);
    }
}
