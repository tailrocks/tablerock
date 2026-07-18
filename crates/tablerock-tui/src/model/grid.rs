//! Presentation-local data grid model for VirtualGrid composition.
//!
//! Cell projections are computed by the CLI/engine bridge when pages admit;
//! the TUI never decodes page arenas.

/// Visual distinction class (text+glyph; never color alone).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellDistinction {
    Null,
    Empty,
    Boolean,
    Number,
    Text,
    Temporal,
    Structured,
    Binary,
    Truncated,
    Invalid,
    Unknown,
    Pending,
}

impl CellDistinction {
    #[must_use]
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Null => "∅",
            Self::Empty => "·",
            Self::Boolean => "",
            Self::Number => "",
            Self::Text => "",
            Self::Temporal => "",
            Self::Structured => "{}",
            Self::Binary => "⟨b⟩",
            Self::Truncated => "…",
            Self::Invalid => "!",
            Self::Unknown => "?",
            Self::Pending => "…",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Empty => "empty",
            Self::Boolean => "bool",
            Self::Number => "number",
            Self::Text => "text",
            Self::Temporal => "time",
            Self::Structured => "structured",
            Self::Binary => "binary",
            Self::Truncated => "truncated",
            Self::Invalid => "invalid",
            Self::Unknown => "unknown",
            Self::Pending => "pending",
        }
    }
}

/// One projected grid cell ready for VirtualGrid paint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedCell {
    pub text: String,
    pub distinction: CellDistinction,
    pub byte_len: u64,
    pub original_byte_len: Option<u64>,
}

impl ProjectedCell {
    #[must_use]
    pub fn pending() -> Self {
        Self {
            text: "…".into(),
            distinction: CellDistinction::Pending,
            byte_len: 0,
            original_byte_len: None,
        }
    }

    #[must_use]
    pub fn display(&self) -> String {
        let glyph = self.distinction.glyph();
        if glyph.is_empty() {
            self.text.clone()
        } else if self.text.is_empty() {
            glyph.into()
        } else {
            format!("{glyph} {}", self.text)
        }
    }
}

/// Operation state machine projection for status bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridOperationState {
    #[default]
    Idle,
    Queued,
    Running,
    Streaming,
    Completed,
    CancelRequested,
    Cancelled,
    Failed,
    Disconnected,
}

impl GridOperationState {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Streaming => "streaming",
            Self::Completed => "completed",
            Self::CancelRequested => "cancel requested",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
            Self::Disconnected => "disconnected",
        }
    }
}

/// Totals fact for the active result.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum GridRowTotal {
    #[default]
    Unknown,
    Exact(u64),
    Estimated(u64),
}

impl GridRowTotal {
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Unknown => "total unknown".into(),
            Self::Exact(n) => format!("total {n}"),
            Self::Estimated(n) => format!("~{n} rows"),
        }
    }
}

/// Resident projected window for one grid tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataGridModel {
    pub columns: Vec<String>,
    /// Absolute start row of the resident matrix.
    pub start_row: u64,
    /// Row-major projected cells: len = rows * columns.
    pub cells: Vec<ProjectedCell>,
    pub row_count: u32,
    pub totals: GridRowTotal,
    pub operation: GridOperationState,
    pub rows_loaded: u64,
    pub bytes_loaded: u64,
    pub truncated: bool,
    pub error_label: Option<String>,
    pub result_token: u64,
    pub cursor_row: u64,
    pub cursor_col: usize,
    /// First visible row in the VirtualGrid viewport (absolute).
    pub viewport_row: u64,
    pub viewport_col: usize,
}

impl Default for DataGridModel {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            start_row: 0,
            cells: Vec::new(),
            row_count: 0,
            totals: GridRowTotal::Unknown,
            operation: GridOperationState::Idle,
            rows_loaded: 0,
            bytes_loaded: 0,
            truncated: false,
            error_label: None,
            result_token: 0,
            cursor_row: 0,
            cursor_col: 0,
            viewport_row: 0,
            viewport_col: 0,
        }
    }
}

impl DataGridModel {
    #[must_use]
    pub fn cell_at(&self, abs_row: u64, col: usize) -> ProjectedCell {
        if col >= self.columns.len() {
            return ProjectedCell::pending();
        }
        if abs_row < self.start_row {
            return ProjectedCell::pending();
        }
        let local = abs_row - self.start_row;
        if local >= u64::from(self.row_count) {
            return ProjectedCell::pending();
        }
        let index = (local as usize)
            .saturating_mul(self.columns.len())
            .saturating_add(col);
        self.cells
            .get(index)
            .cloned()
            .unwrap_or_else(ProjectedCell::pending)
    }

    pub fn replace_page(
        &mut self,
        start_row: u64,
        columns: Vec<String>,
        cells: Vec<ProjectedCell>,
        row_count: u32,
        totals: GridRowTotal,
        bytes: u64,
        truncated: bool,
    ) {
        self.start_row = start_row;
        self.columns = columns;
        self.cells = cells;
        self.row_count = row_count;
        self.totals = totals;
        self.bytes_loaded = self.bytes_loaded.saturating_add(bytes);
        self.rows_loaded = self
            .rows_loaded
            .max(start_row.saturating_add(u64::from(row_count)));
        self.truncated = self.truncated || truncated;
        if matches!(
            self.operation,
            GridOperationState::Idle | GridOperationState::Queued | GridOperationState::Running
        ) {
            self.operation = GridOperationState::Streaming;
        }
    }

    pub fn mark_completed(&mut self) {
        self.operation = GridOperationState::Completed;
    }

    pub fn mark_failed(&mut self, label: impl Into<String>) {
        self.operation = GridOperationState::Failed;
        self.error_label = Some(label.into());
    }

    pub fn mark_cancel_requested(&mut self) {
        self.operation = GridOperationState::CancelRequested;
    }

    pub fn mark_cancelled(&mut self) {
        self.operation = GridOperationState::Cancelled;
    }

    #[must_use]
    pub fn status_line(&self) -> String {
        let trunc = if self.truncated { " trunc" } else { "" };
        let err = self
            .error_label
            .as_deref()
            .map(|e| format!(" · {e}"))
            .unwrap_or_default();
        format!(
            "{} · {} rows · {} B · {}{}{err}",
            self.operation.label(),
            self.rows_loaded,
            self.bytes_loaded,
            self.totals.label(),
            trunc
        )
    }

    /// True when abs_row is inside the resident window (no fetch needed).
    #[must_use]
    pub fn is_resident(&self, abs_row: u64) -> bool {
        abs_row >= self.start_row
            && abs_row < self.start_row.saturating_add(u64::from(self.row_count))
    }
}

/// Project a kind label (from bridge) into a distinction class.
#[must_use]
pub fn distinction_from_kind_label(
    kind: &str,
    is_null: bool,
    truncated: bool,
    empty: bool,
) -> CellDistinction {
    if is_null {
        return CellDistinction::Null;
    }
    if truncated {
        return CellDistinction::Truncated;
    }
    if empty {
        return CellDistinction::Empty;
    }
    match kind {
        "boolean" | "bool" => CellDistinction::Boolean,
        "signed" | "unsigned" | "float64" | "decimal" | "number" => CellDistinction::Number,
        "text" => CellDistinction::Text,
        "temporal" | "time" => CellDistinction::Temporal,
        "structured" | "json" => CellDistinction::Structured,
        "binary" => CellDistinction::Binary,
        "invalid" => CellDistinction::Invalid,
        "pending" => CellDistinction::Pending,
        _ => CellDistinction::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_distinction_has_glyph_or_text_treatment() {
        for d in [
            CellDistinction::Null,
            CellDistinction::Empty,
            CellDistinction::Boolean,
            CellDistinction::Number,
            CellDistinction::Text,
            CellDistinction::Temporal,
            CellDistinction::Structured,
            CellDistinction::Binary,
            CellDistinction::Truncated,
            CellDistinction::Invalid,
            CellDistinction::Unknown,
            CellDistinction::Pending,
        ] {
            assert!(!d.label().is_empty());
            let cell = ProjectedCell {
                text: "x".into(),
                distinction: d,
                byte_len: 1,
                original_byte_len: None,
            };
            assert!(!cell.display().is_empty());
        }
    }

    #[test]
    fn resident_window_and_pending_outside() {
        let mut grid = DataGridModel::default();
        grid.replace_page(
            10,
            vec!["a".into(), "b".into()],
            vec![
                ProjectedCell {
                    text: "1".into(),
                    distinction: CellDistinction::Number,
                    byte_len: 1,
                    original_byte_len: None,
                },
                ProjectedCell {
                    text: "x".into(),
                    distinction: CellDistinction::Text,
                    byte_len: 1,
                    original_byte_len: None,
                },
            ],
            1,
            GridRowTotal::Exact(100),
            2,
            false,
        );
        assert!(grid.is_resident(10));
        assert!(!grid.is_resident(9));
        assert!(!grid.is_resident(11));
        assert_eq!(grid.cell_at(10, 0).text, "1");
        assert_eq!(grid.cell_at(11, 0).distinction, CellDistinction::Pending);
    }

    #[test]
    fn status_line_includes_operation_and_totals() {
        let mut grid = DataGridModel::default();
        grid.operation = GridOperationState::Streaming;
        grid.rows_loaded = 500;
        grid.bytes_loaded = 4096;
        grid.totals = GridRowTotal::Estimated(2500);
        let line = grid.status_line();
        assert!(line.contains("streaming"));
        assert!(line.contains("500"));
        assert!(line.contains("~2500"));
    }
}
