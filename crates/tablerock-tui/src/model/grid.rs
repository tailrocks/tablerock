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

/// Column sort cycle for header clicks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColumnSort {
    #[default]
    None,
    Asc,
    Desc,
}

impl ColumnSort {
    #[must_use]
    pub const fn cycle(self) -> Self {
        match self {
            Self::None => Self::Asc,
            Self::Asc => Self::Desc,
            Self::Desc => Self::None,
        }
    }

    #[must_use]
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Asc => "↑",
            Self::Desc => "↓",
        }
    }
}

/// One sort key in multi-column order (index 0 = primary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridSortKey {
    pub column: String,
    pub direction: ColumnSort,
}

/// Typed filter chip (values as display strings; engine re-types).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridFilterChip {
    pub column: String,
    pub operator: String,
    pub value: Option<String>,
}

/// Column visibility/width layout for one grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnLayout {
    pub name: String,
    pub visible: bool,
    pub width: u16,
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
    /// Server sort keys (provenance for status bar).
    pub sort: Vec<GridSortKey>,
    /// Typed server filters (re-run query).
    pub filters: Vec<GridFilterChip>,
    /// Optional raw WHERE fragment (fail-closed on plan builder).
    pub raw_where: Option<String>,
    /// Page-local quick filter (never emits I/O).
    pub quick_filter: String,
    /// Per-column layout; empty means default widths/all visible.
    pub column_layout: Vec<ColumnLayout>,
    /// Base table identity for SQL INSERT/UPDATE copy (browse only).
    pub base_schema: Option<String>,
    pub base_table: Option<String>,
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
            sort: Vec::new(),
            filters: Vec::new(),
            raw_where: None,
            quick_filter: String::new(),
            column_layout: Vec::new(),
            base_schema: None,
            base_table: None,
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
        let sort = if self.sort.is_empty() {
            String::new()
        } else {
            let parts: Vec<_> = self
                .sort
                .iter()
                .map(|k| format!("{}{}", k.column, k.direction.glyph()))
                .collect();
            format!(" · sort {}", parts.join(","))
        };
        let filt = if self.filters.is_empty() && self.raw_where.is_none() {
            String::new()
        } else {
            format!(
                " · filters {}",
                self.filters.len() + usize::from(self.raw_where.is_some())
            )
        };
        let quick = if self.quick_filter.is_empty() {
            String::new()
        } else {
            " · page-local filter".into()
        };
        format!(
            "{} · {} rows · {} B · {}{}{sort}{filt}{quick}{err}",
            self.operation.label(),
            self.rows_loaded,
            self.bytes_loaded,
            self.totals.label(),
            trunc
        )
    }

    /// Cycle sort on `column` as primary (removes prior keys for that column).
    pub fn cycle_sort_column(&mut self, column: &str) {
        let existing = self.sort.iter().position(|k| k.column == column);
        if let Some(idx) = existing {
            let next = self.sort[idx].direction.cycle();
            if matches!(next, ColumnSort::None) {
                self.sort.remove(idx);
            } else {
                self.sort[idx].direction = next;
                // Move to primary.
                let key = self.sort.remove(idx);
                self.sort.insert(0, key);
            }
        } else {
            self.sort.insert(
                0,
                GridSortKey {
                    column: column.to_owned(),
                    direction: ColumnSort::Asc,
                },
            );
        }
    }

    /// Clear server sort/filter; keep quick filter (page-local).
    pub fn clear_server_controls(&mut self) {
        self.sort.clear();
        self.filters.clear();
        self.raw_where = None;
    }

    /// Visible column names in display order.
    #[must_use]
    pub fn visible_columns(&self) -> Vec<String> {
        if self.column_layout.is_empty() {
            return self.columns.clone();
        }
        self.column_layout
            .iter()
            .filter(|c| c.visible)
            .map(|c| c.name.clone())
            .collect()
    }

    /// Ensure layout entries exist for all columns (default width 12, visible).
    pub fn ensure_column_layout(&mut self) {
        if self.column_layout.is_empty() {
            self.column_layout = self
                .columns
                .iter()
                .map(|name| ColumnLayout {
                    name: name.clone(),
                    visible: true,
                    width: 12,
                })
                .collect();
            return;
        }
        for name in &self.columns {
            if !self.column_layout.iter().any(|c| c.name == *name) {
                self.column_layout.push(ColumnLayout {
                    name: name.clone(),
                    visible: true,
                    width: 12,
                });
            }
        }
    }

    pub fn reset_column_layout(&mut self) {
        self.column_layout.clear();
        self.ensure_column_layout();
    }

    /// Rows matching page-local quick filter (no I/O).
    #[must_use]
    pub fn quick_filter_matches(&self) -> Vec<u64> {
        if self.quick_filter.is_empty() {
            return (self.start_row..self.start_row.saturating_add(u64::from(self.row_count)))
                .collect();
        }
        let needle = self.quick_filter.to_ascii_lowercase();
        let mut out = Vec::new();
        for local in 0..self.row_count {
            let abs = self.start_row.saturating_add(u64::from(local));
            let mut hit = false;
            for col in 0..self.columns.len() {
                let text = self.cell_at(abs, col).display().to_ascii_lowercase();
                if text.contains(&needle) {
                    hit = true;
                    break;
                }
            }
            if hit {
                out.push(abs);
            }
        }
        out
    }

    /// True when abs_row is inside the resident window (no fetch needed).
    #[must_use]
    pub fn is_resident(&self, abs_row: u64) -> bool {
        abs_row >= self.start_row
            && abs_row < self.start_row.saturating_add(u64::from(self.row_count))
    }

    /// Absolute row just past the resident window (for FetchPage).
    #[must_use]
    pub fn next_fetch_start(&self) -> u64 {
        self.start_row.saturating_add(u64::from(self.row_count))
    }

    /// Whether scrolling to `abs_row` would require a fetch (outside resident).
    #[must_use]
    pub fn needs_fetch(&self, abs_row: u64) -> bool {
        !self.is_resident(abs_row)
            && !matches!(
                self.operation,
                GridOperationState::Idle | GridOperationState::Failed
            )
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

    #[test]
    fn cycle_sort_and_quick_filter_page_local() {
        let mut g = DataGridModel::default();
        g.columns = vec!["id".into(), "name".into()];
        g.row_count = 2;
        g.cells = vec![
            ProjectedCell {
                text: "1".into(),
                distinction: CellDistinction::Number,
                byte_len: 1,
                original_byte_len: None,
            },
            ProjectedCell {
                text: "alpha".into(),
                distinction: CellDistinction::Text,
                byte_len: 5,
                original_byte_len: None,
            },
            ProjectedCell {
                text: "2".into(),
                distinction: CellDistinction::Number,
                byte_len: 1,
                original_byte_len: None,
            },
            ProjectedCell {
                text: "beta".into(),
                distinction: CellDistinction::Text,
                byte_len: 4,
                original_byte_len: None,
            },
        ];
        g.cycle_sort_column("name");
        assert_eq!(g.sort.len(), 1);
        assert_eq!(g.sort[0].direction, ColumnSort::Asc);
        g.cycle_sort_column("name");
        assert_eq!(g.sort[0].direction, ColumnSort::Desc);
        g.quick_filter = "alp".into();
        let hits = g.quick_filter_matches();
        assert_eq!(hits, vec![0]);
        assert!(g.status_line().contains("page-local filter"));
        assert!(g.status_line().contains("sort"));
    }

    #[test]
    fn needs_fetch_only_outside_resident_while_active() {
        let mut grid = DataGridModel::default();
        grid.replace_page(
            0,
            vec!["a".into()],
            vec![ProjectedCell {
                text: "1".into(),
                distinction: CellDistinction::Number,
                byte_len: 1,
                original_byte_len: None,
            }],
            1,
            GridRowTotal::Unknown,
            1,
            false,
        );
        assert!(!grid.needs_fetch(0));
        assert!(grid.needs_fetch(1));
        assert_eq!(grid.next_fetch_start(), 1);
        grid.operation = GridOperationState::Idle;
        assert!(!grid.needs_fetch(1));
    }
}
