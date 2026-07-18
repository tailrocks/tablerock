//! Presentation-local data grid model for VirtualGrid composition.
//!
//! Cell projections are computed by the CLI/engine bridge when pages admit;
//! the TUI never decodes page arenas.

use super::mutation_draft::{DraftLocatorField, MutationDraftModel, StagedCellEdit};
use tablerock_core::EditabilityFacts;

/// In-grid cell edit session (presentation only; commit stages a draft).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellEditSession {
    pub abs_row: u64,
    pub column: String,
    pub original_text: String,
    pub buffer: String,
    pub locator: Vec<DraftLocatorField>,
    /// Distinction class of the cell when edit began (drives type-specific UX).
    pub kind: CellDistinction,
}

impl CellEditSession {
    /// Cycle boolean buffer true ↔ false (no-op for other kinds).
    pub fn toggle_boolean(&mut self) -> bool {
        if self.kind != CellDistinction::Boolean {
            return false;
        }
        let t = self.buffer.trim();
        self.buffer = if t.eq_ignore_ascii_case("true") || t == "t" || t == "1" {
            "false".into()
        } else {
            "true".into()
        };
        true
    }

    /// Set buffer to SQL NULL presentation token.
    pub fn set_null(&mut self) {
        self.buffer = "null".into();
    }

    /// Stamp local calendar date `YYYY-MM-DD` for temporal cells.
    pub fn set_today(&mut self) -> bool {
        if self.kind != CellDistinction::Temporal {
            return false;
        }
        self.buffer = local_today_iso();
        true
    }

    /// Stamp local timestamp `YYYY-MM-DDTHH:MM:SS` for temporal cells.
    pub fn set_now(&mut self) -> bool {
        if self.kind != CellDistinction::Temporal {
            return false;
        }
        self.buffer = local_now_iso();
        true
    }

    /// Pretty-indent structured/JSON buffer (best-effort; fail closed on non-JSON).
    pub fn format_structured(&mut self) -> bool {
        if self.kind != CellDistinction::Structured {
            return false;
        }
        let pretty = pretty_json_like(self.buffer.trim());
        if pretty == self.buffer {
            return false;
        }
        self.buffer = pretty;
        true
    }

    /// Compact structured buffer to a single line (best-effort).
    pub fn compact_structured(&mut self) -> bool {
        if self.kind != CellDistinction::Structured {
            return false;
        }
        let compact = compact_json_like(self.buffer.trim());
        if compact == self.buffer {
            return false;
        }
        self.buffer = compact;
        true
    }

    /// Step integer/float buffer by `delta` for Number cells (presentation only).
    pub fn step_number(&mut self, delta: i64) -> bool {
        if self.kind != CellDistinction::Number || delta == 0 {
            return false;
        }
        let t = self.buffer.trim();
        if t.is_empty() || t.eq_ignore_ascii_case("null") {
            self.buffer = delta.to_string();
            return true;
        }
        if let Ok(n) = t.parse::<i128>() {
            let next = n.saturating_add(i128::from(delta));
            self.buffer = next.to_string();
            return true;
        }
        if let Ok(f) = t.parse::<f64>() {
            if !f.is_finite() {
                return false;
            }
            let next = f + delta as f64;
            if !next.is_finite() {
                return false;
            }
            // Prefer short decimal when step keeps a .0 integer-looking value.
            if next.fract() == 0.0 && next.abs() < 1e15 {
                self.buffer = format!("{}", next as i64);
            } else {
                self.buffer = format!("{next}");
            }
            return true;
        }
        false
    }
}

fn local_today_iso() -> String {
    // Format via chrono-less wall clock: use UTC date from system time for
    // portability without new deps (host local TZ would need OS helpers).
    // Operators still edit freely; this is a staging affordance only.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

fn local_now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400) as u32;
    let (y, m, d) = civil_from_days(days);
    let hh = tod / 3600;
    let mm = (tod % 3600) / 60;
    let ss = tod % 60;
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Best-effort JSON-like pretty print (objects/arrays only). Non-JSON returns input.
fn pretty_json_like(raw: &str) -> String {
    let trimmed = raw.trim();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return raw.to_owned();
    }
    let mut out = String::with_capacity(trimmed.len() + 32);
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut escape = false;
    let bytes = trimmed.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if out.len() > 16 * 1024 {
            out.push_str("\n…");
            break;
        }
        let b = bytes[i];
        if in_str {
            out.push(b as char);
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' => {
                in_str = true;
                out.push('"');
            }
            b'{' | b'[' => {
                out.push(b as char);
                depth += 1;
                out.push('\n');
                for _ in 0..depth {
                    out.push_str("  ");
                }
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                out.push('\n');
                for _ in 0..depth {
                    out.push_str("  ");
                }
                out.push(b as char);
            }
            b',' => {
                out.push(',');
                out.push('\n');
                for _ in 0..depth {
                    out.push_str("  ");
                }
                if bytes.get(i + 1) == Some(&b' ') {
                    i += 1;
                }
            }
            b':' => {
                out.push(':');
                out.push(' ');
                if bytes.get(i + 1) == Some(&b' ') {
                    i += 1;
                }
            }
            b' ' | b'\n' | b'\t' | b'\r' => {}
            _ => out.push(b as char),
        }
        i += 1;
    }
    out
}

/// Collapse JSON-like whitespace outside strings.
fn compact_json_like(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_str = false;
    let mut escape = false;
    for b in raw.bytes() {
        if in_str {
            out.push(b as char);
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => {
                in_str = true;
                out.push('"');
            }
            b' ' | b'\n' | b'\t' | b'\r' => {}
            _ => out.push(b as char),
        }
    }
    out
}

/// Howard Hinnant civil-from-days (proleptic Gregorian).
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

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
    /// Cancel requested; dispatch not yet classified.
    CancelRequested,
    /// Client stopped consuming (local) without server confirm.
    ClientStopped,
    /// Server confirmed cancel (KILL QUERY finished / server cancel).
    ServerConfirmedCancelled,
    /// Cancel outcome unknown (transport loss / incomplete).
    CancelUnknown,
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
            Self::ClientStopped => "client stopped",
            Self::ServerConfirmedCancelled => "server confirmed cancelled",
            Self::CancelUnknown => "cancel unknown",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
            Self::Disconnected => "disconnected",
        }
    }

    /// True when an in-flight operation should flip to Disconnected on session loss.
    #[must_use]
    pub const fn is_live(self) -> bool {
        matches!(
            self,
            Self::Queued
                | Self::Running
                | Self::Streaming
                | Self::CancelRequested
                | Self::ClientStopped
                | Self::CancelUnknown
        )
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
    /// ClickHouse (or other) server query id for cancel/status while running.
    pub server_query_id: Option<String>,
    /// ClickHouse X-ClickHouse-Summary progress (partial without wait_end_of_query).
    pub server_progress: Option<String>,
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
    /// Proven primary/unique key column names (empty = unknown).
    pub identity_columns: Vec<String>,
    /// Result-level editability (drives staging affordances).
    pub editability: EditabilityFacts,
    /// In-memory staged mutations for this tab.
    pub drafts: MutationDraftModel,
    /// Active inline cell edit (None when not editing).
    pub cell_edit: Option<CellEditSession>,
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
            server_query_id: None,
            server_progress: None,
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
            identity_columns: Vec::new(),
            editability: EditabilityFacts::ReadOnly {
                reason: tablerock_core::EditabilityReason::NoBaseTable,
            },
            drafts: MutationDraftModel::new(),
            cell_edit: None,
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

    /// Map cancel-dispatch fact to a distinct presentation state.
    pub fn mark_cancel_dispatch(&mut self, dispatch_label: &str) {
        self.operation = match dispatch_label {
            "request_sent" | "requested" => GridOperationState::CancelRequested,
            "prevented" | "client_stopped" => GridOperationState::ClientStopped,
            "server_rejected" | "transport_failed" | "unknown" | "unsupported" => {
                GridOperationState::CancelUnknown
            }
            "server_confirmed" => GridOperationState::ServerConfirmedCancelled,
            _ => GridOperationState::CancelRequested,
        };
        self.error_label = Some(format!("cancel: {dispatch_label}"));
    }

    pub fn mark_cancelled(&mut self) {
        self.operation = GridOperationState::Cancelled;
    }

    pub fn mark_server_confirmed_cancelled(&mut self) {
        self.operation = GridOperationState::ServerConfirmedCancelled;
    }

    /// Session loss: terminal for live ops; completed/failed content stays inspectable.
    pub fn mark_disconnected(&mut self) {
        if self.operation.is_live() {
            self.operation = GridOperationState::Disconnected;
        }
    }

    #[must_use]
    pub fn status_line(&self) -> String {
        let trunc = if self.truncated { " trunc" } else { "" };
        let err = self
            .error_label
            .as_deref()
            .map(|e| format!(" · {e}"))
            .unwrap_or_default();
        let qid = self
            .server_query_id
            .as_deref()
            .map(|id| format!(" · qid {id}"))
            .unwrap_or_default();
        let progress = self
            .server_progress
            .as_deref()
            .map(|p| format!(" · {p}"))
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
        let filt = self.filter_status_suffix();
        let quick = if self.quick_filter.is_empty() {
            String::new()
        } else {
            format!(" · page-local [{}]", self.quick_filter)
        };
        let staged = self.drafts.status_suffix();
        let edit = if self.editability.is_editable() {
            String::new()
        } else if let Some(reason) = self.editability.reason() {
            format!(" · read-only ({})", reason.label())
        } else {
            String::new()
        };
        format!(
            "{} · {} rows · {} B · {}{}{qid}{progress}{sort}{filt}{quick}{staged}{edit}{err}",
            self.operation.label(),
            self.rows_loaded,
            self.bytes_loaded,
            self.totals.label(),
            trunc
        )
    }

    /// Visual filter chip bar for the workbench (glyph + text, never color alone).
    ///
    /// Empty when no server filters, raw WHERE, or page-local quick filter.
    #[must_use]
    pub fn filter_chip_bar(&self) -> Option<String> {
        let mut chips: Vec<String> = self
            .filters
            .iter()
            .take(12)
            .map(|f| match f.value.as_deref() {
                Some(v) if !v.is_empty() => format!("[{} {} {}]", f.column, f.operator, v),
                _ => format!("[{} {}]", f.column, f.operator),
            })
            .collect();
        if let Some(raw) = self.raw_where.as_deref() {
            if !raw.is_empty() {
                let clipped: String = raw.chars().take(48).collect();
                chips.push(format!("[WHERE {clipped}]"));
            }
        }
        if !self.quick_filter.is_empty() {
            chips.push(format!("[page:{}]", self.quick_filter));
        }
        if chips.is_empty() {
            return None;
        }
        if self.filters.len() > 12 {
            chips.push(format!("[+{} more]", self.filters.len() - 12));
        }
        Some(format!("filters: {}", chips.join(" ")))
    }

    fn filter_status_suffix(&self) -> String {
        if self.filters.is_empty() && self.raw_where.is_none() {
            return String::new();
        }
        let parts: Vec<String> = self
            .filters
            .iter()
            .take(4)
            .map(|f| match f.value.as_deref() {
                Some(v) if !v.is_empty() => format!("{}{}{}", f.column, f.operator, v),
                _ => format!("{}{}", f.column, f.operator),
            })
            .collect();
        let mut s = format!(" · filters {}", parts.join(","));
        if self.filters.len() > 4 {
            s.push_str(&format!(" +{}", self.filters.len() - 4));
        }
        if self.raw_where.is_some() {
            s.push_str(" +WHERE");
        }
        s
    }

    /// Recompute editability from base identity + profile safety + shape flag.
    pub fn recompute_editability(
        &mut self,
        safety: tablerock_core::ProfileSafetyMode,
        non_base: bool,
    ) {
        self.editability = EditabilityFacts::classify(
            safety,
            non_base,
            self.base_schema.as_deref(),
            self.base_table.as_deref(),
            &self.identity_columns,
        );
        self.drafts.apply_editability(&self.editability);
        if !self.drafts.staging_allowed() {
            self.cell_edit = None;
        }
    }

    /// Begin editing the cursor cell when the result is editable.
    pub fn begin_cell_edit(&mut self) -> bool {
        if !self.drafts.staging_allowed() || !self.editability.is_editable() {
            return false;
        }
        if self.columns.is_empty() {
            return false;
        }
        let col_idx = self.cursor_col.min(self.columns.len().saturating_sub(1));
        let column = self.columns[col_idx].clone();
        // Identity columns themselves are locators; still allow edit of non-key cells.
        let cell = self.cell_at(self.cursor_row, col_idx);
        if matches!(
            cell.distinction,
            CellDistinction::Truncated | CellDistinction::Invalid | CellDistinction::Unknown
        ) {
            return false;
        }
        let locator = self.locator_for_row(self.cursor_row);
        if locator.is_empty() {
            return false;
        }
        self.cell_edit = Some(CellEditSession {
            abs_row: self.cursor_row,
            column,
            original_text: cell.text.clone(),
            buffer: cell.text,
            locator,
            kind: cell.distinction,
        });
        true
    }

    /// Build locator fields from identity columns at `abs_row`.
    #[must_use]
    pub fn locator_for_row(&self, abs_row: u64) -> Vec<DraftLocatorField> {
        let mut out = Vec::new();
        for name in &self.identity_columns {
            let Some(col_idx) = self.columns.iter().position(|c| c == name) else {
                return Vec::new();
            };
            let cell = self.cell_at(abs_row, col_idx);
            out.push(DraftLocatorField {
                column: name.clone(),
                original_text: cell.text,
            });
        }
        out
    }

    /// Commit active cell edit into drafts. Returns true if a draft was staged.
    ///
    /// Validates staged text against the original cell's distinction class
    /// (bool/number/text). Truncated/invalid/unknown are never commit-able.
    pub fn commit_cell_edit(&mut self) -> bool {
        let Some(session) = self.cell_edit.take() else {
            return false;
        };
        let col_idx = self.columns.iter().position(|c| c == &session.column);
        if let Some(idx) = col_idx {
            let cell = self.cell_at(session.abs_row, idx);
            if !staged_value_ok_for_distinction(&session.buffer, cell.distinction) {
                // Restore session so operator can fix the buffer.
                self.cell_edit = Some(session);
                return false;
            }
        }
        self.drafts.stage_cell_edit(StagedCellEdit {
            abs_row: session.abs_row,
            column: session.column,
            original_text: session.original_text,
            staged_text: session.buffer,
            locator: session.locator,
        })
    }

    pub fn cancel_cell_edit(&mut self) {
        self.cell_edit = None;
    }

    /// Stage delete for the cursor row.
    pub fn stage_delete_cursor_row(&mut self) -> bool {
        if !self.drafts.staging_allowed() {
            return false;
        }
        let locator = self.locator_for_row(self.cursor_row);
        if locator.is_empty() {
            return false;
        }
        self.drafts.stage_delete(super::mutation_draft::StagedDelete {
            abs_row: self.cursor_row,
            locator,
        })
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

    /// Toggle visibility of a named column (at least one remains visible).
    pub fn toggle_column_visible(&mut self, column: &str) -> bool {
        self.ensure_column_layout();
        let visible_count = self.column_layout.iter().filter(|c| c.visible).count();
        if let Some(entry) = self.column_layout.iter_mut().find(|c| c.name == column) {
            if entry.visible && visible_count <= 1 {
                return false;
            }
            entry.visible = !entry.visible;
            return true;
        }
        false
    }

    /// Physical column index for a layout/display name.
    #[must_use]
    pub fn physical_column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }

    /// Move the cursor's column one step in display layout order (`dir` = -1 left, +1 right).
    pub fn move_cursor_column(&mut self, dir: i8) -> bool {
        if dir == 0 {
            return false;
        }
        self.ensure_column_layout();
        let Some(name) = self.columns.get(self.cursor_col).cloned() else {
            return false;
        };
        let Some(idx) = self.column_layout.iter().position(|c| c.name == name) else {
            return false;
        };
        let target = if dir < 0 {
            idx.checked_sub(1)
        } else {
            idx.checked_add(1)
        };
        let Some(target) = target.filter(|&t| t < self.column_layout.len()) else {
            return false;
        };
        self.column_layout.swap(idx, target);
        true
    }

    /// Adjust width of the cursor column in layout (`delta` usually ±2). Bounds 4..=64.
    pub fn adjust_cursor_column_width(&mut self, delta: i16) -> bool {
        if delta == 0 {
            return false;
        }
        self.ensure_column_layout();
        let Some(name) = self.columns.get(self.cursor_col).cloned() else {
            return false;
        };
        let Some(entry) = self.column_layout.iter_mut().find(|c| c.name == name) else {
            return false;
        };
        let next = i32::from(entry.width) + i32::from(delta);
        let clamped = next.clamp(4, 64) as u16;
        if clamped == entry.width {
            return false;
        }
        entry.width = clamped;
        true
    }

    /// Fit cursor column width to resident content + header (bounds 4..=64).
    pub fn fit_cursor_column(&mut self) -> bool {
        self.ensure_column_layout();
        let Some(name) = self.columns.get(self.cursor_col).cloned() else {
            return false;
        };
        let fitted = self.measure_column_width(&name);
        let Some(entry) = self.column_layout.iter_mut().find(|c| c.name == name) else {
            return false;
        };
        if entry.width == fitted {
            return false;
        }
        entry.width = fitted;
        true
    }

    /// Fit every visible column from resident content.
    pub fn fit_all_visible_columns(&mut self) -> bool {
        self.ensure_column_layout();
        let names: Vec<String> = self
            .column_layout
            .iter()
            .filter(|c| c.visible)
            .map(|c| c.name.clone())
            .collect();
        if names.is_empty() {
            return false;
        }
        let mut changed = false;
        for name in names {
            let fitted = self.measure_column_width(&name);
            if let Some(entry) = self.column_layout.iter_mut().find(|c| c.name == name) {
                if entry.width != fitted {
                    entry.width = fitted;
                    changed = true;
                }
            }
        }
        changed
    }

    /// Measure display width for a column from header + resident cells.
    fn measure_column_width(&self, name: &str) -> u16 {
        let mut max_w = name.chars().count().max(1);
        let Some(phys) = self.physical_column_index(name) else {
            return 12;
        };
        for local in 0..self.row_count as usize {
            let abs = self.start_row.saturating_add(local as u64);
            let display = self.cell_at(abs, phys).display();
            // Cap per-cell contribution so one huge blob does not force max width alone
            // beyond the global 64 clamp.
            let w = display.chars().count().min(64);
            if w > max_w {
                max_w = w;
            }
        }
        u16::try_from(max_w).unwrap_or(64).clamp(4, 64)
    }

    /// Layout width for a column name (default 12).
    #[must_use]
    pub fn column_width(&self, name: &str) -> u16 {
        self.column_layout
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.width.max(4))
            .unwrap_or(12)
    }

    /// Serialize layout for persistence (names/visible/width only).
    #[must_use]
    pub fn layout_json(&self) -> String {
        let mut out = String::from("[");
        for (i, c) in self.column_layout.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                r#"{{"name":"{}","visible":{},"width":{}}}"#,
                c.name.replace('\\', "\\\\").replace('"', "\\\""),
                c.visible,
                c.width
            ));
        }
        out.push(']');
        out
    }

    /// Apply layout JSON from persistence.
    pub fn apply_layout_json(&mut self, json: &str) -> bool {
        if json.contains("\"cells\"") {
            return false;
        }
        let mut layout = Vec::new();
        let mut rest = json;
        while let Some(idx) = rest.find("\"name\"") {
            rest = &rest[idx..];
            let Some(name) = extract_layout_string(rest, "name") else {
                break;
            };
            let slice_end = rest.find('}').unwrap_or(rest.len());
            let obj = &rest[..slice_end];
            let visible = !obj.contains("\"visible\":false");
            let width = extract_layout_number(obj, "width").unwrap_or(12) as u16;
            layout.push(ColumnLayout {
                name,
                visible,
                width: width.max(4),
            });
            rest = rest.get(slice_end.saturating_add(1)..).unwrap_or("");
        }
        if layout.is_empty() {
            return false;
        }
        self.column_layout = layout;
        true
    }

    pub fn add_filter_chip(&mut self, column: impl Into<String>, operator: impl Into<String>, value: Option<String>) {
        self.filters.push(GridFilterChip {
            column: column.into(),
            operator: operator.into(),
            value,
        });
    }
}

/// Kind-aware staged text gate (presentation; engine re-types for apply).
fn staged_value_ok_for_distinction(text: &str, distinction: CellDistinction) -> bool {
    let t = text.trim();
    match distinction {
        CellDistinction::Truncated | CellDistinction::Invalid | CellDistinction::Unknown => false,
        CellDistinction::Boolean => {
            t.eq_ignore_ascii_case("true")
                || t.eq_ignore_ascii_case("false")
                || t.eq_ignore_ascii_case("null")
                || t.is_empty()
        }
        CellDistinction::Number => {
            t.is_empty()
                || t.eq_ignore_ascii_case("null")
                || t.parse::<i64>().is_ok()
                || t.parse::<f64>().is_ok()
        }
        CellDistinction::Temporal => is_plausible_temporal(t),
        CellDistinction::Structured => is_plausible_structured(t),
        // Binary stays free-form hex/escape text at this residual; server validates.
        CellDistinction::Null
        | CellDistinction::Empty
        | CellDistinction::Text
        | CellDistinction::Binary
        | CellDistinction::Pending => true,
    }
}

/// Plausible ISO-ish temporal forms for staging (server remains authority).
///
/// Accepts empty/null, date `YYYY-MM-DD`, time `HH:MM[:SS]`, datetime with
/// optional `T`/` ` separator and optional trailing `Z` / `±HH:MM`.
fn is_plausible_temporal(t: &str) -> bool {
    if t.is_empty() || t.eq_ignore_ascii_case("null") {
        return true;
    }
    // Reject control characters / injection-ish noise in staged temporal text.
    if t.chars().any(|c| c.is_control() || c == ';' || c == '\n') {
        return false;
    }
    let bytes = t.as_bytes();
    // Date only: YYYY-MM-DD
    if bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..10].iter().all(u8::is_ascii_digit)
    {
        return true;
    }
    // Time only: HH:MM or HH:MM:SS
    if (bytes.len() == 5 || bytes.len() == 8)
        && bytes[2] == b':'
        && bytes[0..2].iter().all(u8::is_ascii_digit)
        && bytes[3..5].iter().all(u8::is_ascii_digit)
        && (bytes.len() == 5 || (bytes[5] == b':' && bytes[6..8].iter().all(u8::is_ascii_digit)))
    {
        return true;
    }
    // Datetime: date + sep + time [+ frac] [+ zone]
    if bytes.len() >= 16 && bytes[4] == b'-' && bytes[7] == b'-' {
        let sep = bytes[10];
        if sep == b'T' || sep == b' ' {
            let rest = &t[11..];
            // HH:MM at start of rest
            let rb = rest.as_bytes();
            if rb.len() >= 5
                && rb[2] == b':'
                && rb[0..2].iter().all(u8::is_ascii_digit)
                && rb[3..5].iter().all(u8::is_ascii_digit)
            {
                // Remainder may include :SS, .frac, Z, ±offset — digits/colon/dot/plus/minus/Z only.
                return rest.chars().all(|c| {
                    c.is_ascii_digit()
                        || matches!(c, ':' | '.' | '+' | '-' | 'Z' | 'z' | ' ')
                });
            }
        }
    }
    false
}

/// Structured cells: null/empty or JSON object/array/string/number/bool/null token.
fn is_plausible_structured(t: &str) -> bool {
    if t.is_empty() || t.eq_ignore_ascii_case("null") {
        return true;
    }
    let t = t.trim();
    // Bare JSON scalars
    if t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("false") {
        return true;
    }
    if t.parse::<i64>().is_ok() || t.parse::<f64>().is_ok() {
        return true;
    }
    // Quoted string
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        return true;
    }
    // Object / array: balanced braces/brackets, no bare control chars.
    let first = t.as_bytes()[0];
    let last = t.as_bytes()[t.len() - 1];
    if (first == b'{' && last == b'}') || (first == b'[' && last == b']') {
        if t.chars().any(|c| c.is_control() && c != '\n' && c != '\t' && c != '\r') {
            return false;
        }
        return braces_balanced(t);
    }
    false
}

fn braces_balanced(t: &str) -> bool {
    let mut depth = 0_i32;
    let mut in_str = false;
    let mut escape = false;
    for ch in t.chars() {
        if in_str {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                '"' => in_str = false,
                _ => {}
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '{' | '[' => depth += 1,
            '}' | ']' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0 && !in_str
}

fn extract_layout_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let idx = json.find(&needle)?;
    let after = json[idx + needle.len()..].trim_start().strip_prefix(':')?.trim_start();
    let after = after.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = after.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Some(out),
            '\\' => out.push(chars.next()?),
            c => out.push(c),
        }
    }
    None
}

fn extract_layout_number(json: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\"");
    let idx = json.find(&needle)?;
    let after = json[idx + needle.len()..].trim_start().strip_prefix(':')?.trim_start();
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

impl DataGridModel {

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
    fn status_line_includes_query_id_when_set() {
        let mut grid = DataGridModel::default();
        grid.server_query_id = Some("tr-42".into());
        grid.server_progress = Some("read 5 rows".into());
        grid.operation = GridOperationState::Running;
        let line = grid.status_line();
        assert!(line.contains("qid tr-42"), "{line}");
        assert!(line.contains("read 5 rows"), "{line}");
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
    fn layout_json_round_trip_and_toggle() {
        let mut g = DataGridModel::default();
        g.columns = vec!["id".into(), "name".into(), "age".into()];
        g.ensure_column_layout();
        assert!(g.toggle_column_visible("name"));
        let json = g.layout_json();
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"visible\":false"));
        let mut g2 = DataGridModel::default();
        g2.columns = g.columns.clone();
        assert!(g2.apply_layout_json(&json));
        assert_eq!(g2.visible_columns(), vec!["id".to_owned(), "age".to_owned()]);
        // Cannot hide last visible column.
        assert!(g2.toggle_column_visible("id"));
        assert!(!g2.toggle_column_visible("age"));
    }

    #[test]
    fn move_and_resize_cursor_column_in_layout() {
        let mut g = DataGridModel::default();
        g.columns = vec!["id".into(), "name".into(), "age".into()];
        g.cursor_col = 0; // id
        g.ensure_column_layout();
        assert_eq!(
            g.visible_columns(),
            vec!["id".to_owned(), "name".to_owned(), "age".to_owned()]
        );
        assert!(g.move_cursor_column(1));
        assert_eq!(
            g.visible_columns(),
            vec!["name".to_owned(), "id".to_owned(), "age".to_owned()]
        );
        assert!(g.move_cursor_column(1));
        assert_eq!(
            g.visible_columns(),
            vec!["name".to_owned(), "age".to_owned(), "id".to_owned()]
        );
        assert!(!g.move_cursor_column(1)); // already rightmost in layout
        assert_eq!(g.column_width("id"), 12);
        assert!(g.adjust_cursor_column_width(4));
        assert_eq!(g.column_width("id"), 16);
        assert!(g.adjust_cursor_column_width(-100)); // clamp to 4
        assert_eq!(g.column_width("id"), 4);
        assert!(!g.adjust_cursor_column_width(-1)); // already min
        let json = g.layout_json();
        assert!(json.contains("\"width\":4"));
        assert!(json.contains("\"name\":\"id\""));
    }

    #[test]
    fn fit_column_uses_resident_content_and_header() {
        let mut g = DataGridModel::default();
        g.columns = vec!["id".into(), "name".into()];
        g.start_row = 0;
        g.row_count = 2;
        g.cells = vec![
            ProjectedCell {
                text: "1".into(),
                distinction: CellDistinction::Number,
                byte_len: 1,
                original_byte_len: None,
            },
            ProjectedCell {
                text: "abcdefghijklmnop".into(), // 16 chars
                distinction: CellDistinction::Text,
                byte_len: 16,
                original_byte_len: None,
            },
            ProjectedCell {
                text: "99".into(),
                distinction: CellDistinction::Number,
                byte_len: 2,
                original_byte_len: None,
            },
            ProjectedCell {
                text: "xy".into(),
                distinction: CellDistinction::Text,
                byte_len: 2,
                original_byte_len: None,
            },
        ];
        g.cursor_col = 1; // name
        g.ensure_column_layout();
        assert_eq!(g.column_width("name"), 12);
        assert!(g.fit_cursor_column());
        assert_eq!(g.column_width("name"), 16);
        // Second fit is no-op.
        assert!(!g.fit_cursor_column());
        g.cursor_col = 0;
        assert!(g.fit_cursor_column()); // header "id" len 2 → clamp 4
        assert_eq!(g.column_width("id"), 4);
        // Both already fitted → no-op.
        assert!(!g.fit_all_visible_columns());
        // Widen then fit-all restores.
        assert!(g.adjust_cursor_column_width(10));
        assert_eq!(g.column_width("id"), 14);
        assert!(g.fit_all_visible_columns());
        assert_eq!(g.column_width("id"), 4);
        assert_eq!(g.column_width("name"), 16);
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
        assert!(g.status_line().contains("page-local [alp]"));
        assert!(g.status_line().contains("sort"));
    }

    #[test]
    fn filter_chip_bar_lists_server_and_page_filters() {
        let mut g = DataGridModel::default();
        g.add_filter_chip("status", "=", Some("open".into()));
        g.add_filter_chip("amount", ">", Some("10".into()));
        g.raw_where = Some("deleted_at IS NULL".into());
        g.quick_filter = "acme".into();
        let bar = g.filter_chip_bar().expect("chips");
        assert!(bar.contains("[status = open]"), "{bar}");
        assert!(bar.contains("[amount > 10]"), "{bar}");
        assert!(bar.contains("[WHERE deleted_at IS NULL]"), "{bar}");
        assert!(bar.contains("[page:acme]"), "{bar}");
        let status = g.status_line();
        assert!(status.contains("filters status=open"), "{status}");
        assert!(status.contains("+WHERE"), "{status}");
        assert!(status.contains("page-local [acme]"), "{status}");
        assert!(g.filter_chip_bar().is_some());
        g.filters.clear();
        g.raw_where = None;
        g.quick_filter.clear();
        assert!(g.filter_chip_bar().is_none());
    }

    #[test]
    fn recompute_editability_and_staged_status() {
        use super::super::mutation_draft::{DraftLocatorField, StagedCellEdit};
        use tablerock_core::ProfileSafetyMode;

        let mut grid = DataGridModel::default();
        grid.base_schema = Some("public".into());
        grid.base_table = Some("users".into());
        grid.identity_columns = vec!["id".into()];
        grid.recompute_editability(ProfileSafetyMode::ConfirmWrites, false);
        assert!(grid.editability.is_editable());
        assert!(grid.drafts.staging_allowed());
        assert!(grid.drafts.stage_cell_edit(StagedCellEdit {
            abs_row: 0,
            column: "name".into(),
            original_text: "a".into(),
            staged_text: "b".into(),
            locator: vec![DraftLocatorField {
                column: "id".into(),
                original_text: "1".into(),
            }],
        }));
        assert!(grid.status_line().contains("staged 1"));
        grid.recompute_editability(ProfileSafetyMode::ReadOnly, false);
        assert!(!grid.editability.is_editable());
        assert!(grid.drafts.is_empty());
        assert!(grid.status_line().contains("read-only"));
    }

    #[test]
    fn begin_and_commit_cell_edit_stages_draft() {
        use tablerock_core::ProfileSafetyMode;

        let mut grid = DataGridModel::default();
        grid.columns = vec!["id".into(), "name".into()];
        grid.row_count = 1;
        grid.cells = vec![
            ProjectedCell {
                text: "1".into(),
                distinction: CellDistinction::Number,
                byte_len: 1,
                original_byte_len: None,
            },
            ProjectedCell {
                text: "alice".into(),
                distinction: CellDistinction::Text,
                byte_len: 5,
                original_byte_len: None,
            },
        ];
        grid.base_schema = Some("public".into());
        grid.base_table = Some("users".into());
        grid.identity_columns = vec!["id".into()];
        grid.cursor_row = 0;
        grid.cursor_col = 1;
        grid.recompute_editability(ProfileSafetyMode::ConfirmWrites, false);
        assert!(grid.begin_cell_edit());
        if let Some(edit) = grid.cell_edit.as_mut() {
            edit.buffer = "bob".into();
        }
        assert!(grid.commit_cell_edit());
        assert_eq!(grid.drafts.pending_count(), 1);
        assert_eq!(grid.drafts.staged_for_cell(0, "name"), Some("bob"));
    }

    #[test]
    fn number_cell_rejects_non_numeric_stage() {
        use tablerock_core::ProfileSafetyMode;

        let mut grid = DataGridModel::default();
        grid.columns = vec!["id".into(), "n".into()];
        grid.row_count = 1;
        grid.cells = vec![
            ProjectedCell {
                text: "1".into(),
                distinction: CellDistinction::Number,
                byte_len: 1,
                original_byte_len: None,
            },
            ProjectedCell {
                text: "2".into(),
                distinction: CellDistinction::Number,
                byte_len: 1,
                original_byte_len: None,
            },
        ];
        grid.base_schema = Some("public".into());
        grid.base_table = Some("t".into());
        grid.identity_columns = vec!["id".into()];
        grid.cursor_row = 0;
        grid.cursor_col = 1;
        grid.recompute_editability(ProfileSafetyMode::ConfirmWrites, false);
        assert!(grid.begin_cell_edit());
        if let Some(edit) = grid.cell_edit.as_mut() {
            edit.buffer = "not-a-number".into();
        }
        assert!(!grid.commit_cell_edit());
        assert!(grid.cell_edit.is_some());
        if let Some(edit) = grid.cell_edit.as_mut() {
            edit.buffer = "42".into();
        }
        assert!(grid.commit_cell_edit());
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

    #[test]
    fn temporal_set_today_and_now_stamps() {
        let mut session = CellEditSession {
            abs_row: 0,
            column: "ts".into(),
            original_text: String::new(),
            buffer: String::new(),
            locator: Vec::new(),
            kind: CellDistinction::Temporal,
        };
        assert!(session.set_today());
        assert!(staged_value_ok_for_distinction(
            &session.buffer,
            CellDistinction::Temporal
        ));
        assert!(session.buffer.chars().filter(|c| *c == '-').count() >= 2);
        assert!(session.set_now());
        assert!(session.buffer.contains('T'));
        assert!(staged_value_ok_for_distinction(
            &session.buffer,
            CellDistinction::Temporal
        ));
        let mut text = CellEditSession {
            abs_row: 0,
            column: "t".into(),
            original_text: String::new(),
            buffer: "x".into(),
            locator: Vec::new(),
            kind: CellDistinction::Text,
        };
        assert!(!text.set_today());
        assert!(!text.set_now());
    }

    #[test]
    fn number_step_inc_dec() {
        let mut session = CellEditSession {
            abs_row: 0,
            column: "n".into(),
            original_text: "10".into(),
            buffer: "10".into(),
            locator: Vec::new(),
            kind: CellDistinction::Number,
        };
        assert!(session.step_number(1));
        assert_eq!(session.buffer, "11");
        assert!(session.step_number(-3));
        assert_eq!(session.buffer, "8");
        session.buffer = "1.5".into();
        assert!(session.step_number(1));
        assert_eq!(session.buffer, "2.5");
        session.buffer = "null".into();
        assert!(session.step_number(1));
        assert_eq!(session.buffer, "1");
        let mut text = CellEditSession {
            abs_row: 0,
            column: "t".into(),
            original_text: String::new(),
            buffer: "1".into(),
            locator: Vec::new(),
            kind: CellDistinction::Text,
        };
        assert!(!text.step_number(1));
    }

    #[test]
    fn structured_format_and_compact() {
        let mut session = CellEditSession {
            abs_row: 0,
            column: "payload".into(),
            original_text: r#"{"a":1,"b":true}"#.into(),
            buffer: r#"{"a":1,"b":true}"#.into(),
            locator: Vec::new(),
            kind: CellDistinction::Structured,
        };
        assert!(session.format_structured());
        assert!(session.buffer.contains('\n'));
        assert!(session.buffer.contains("\"a\""));
        assert!(session.compact_structured());
        assert!(!session.buffer.contains('\n'));
        assert_eq!(session.buffer, r#"{"a":1,"b":true}"#);
        // Invalid: no change.
        session.buffer = "not-json".into();
        assert!(!session.format_structured());
        assert_eq!(session.buffer, "not-json");
        let mut text = CellEditSession {
            abs_row: 0,
            column: "t".into(),
            original_text: String::new(),
            buffer: r#"{"x":1}"#.into(),
            locator: Vec::new(),
            kind: CellDistinction::Text,
        };
        assert!(!text.format_structured());
        assert!(!text.compact_structured());
    }

    #[test]
    fn temporal_and_structured_staging_validation() {
        assert!(staged_value_ok_for_distinction(
            "2024-02-29",
            CellDistinction::Temporal
        ));
        assert!(staged_value_ok_for_distinction(
            "2024-02-29T12:34:56Z",
            CellDistinction::Temporal
        ));
        assert!(staged_value_ok_for_distinction(
            "12:34:56",
            CellDistinction::Temporal
        ));
        assert!(!staged_value_ok_for_distinction(
            "not-a-date",
            CellDistinction::Temporal
        ));
        assert!(!staged_value_ok_for_distinction(
            "2024-02-29;drop",
            CellDistinction::Temporal
        ));
        assert!(staged_value_ok_for_distinction(
            r#"{"a":1}"#,
            CellDistinction::Structured
        ));
        assert!(staged_value_ok_for_distinction(
            "[1,2,3]",
            CellDistinction::Structured
        ));
        assert!(staged_value_ok_for_distinction(
            "null",
            CellDistinction::Structured
        ));
        assert!(!staged_value_ok_for_distinction(
            "{bad",
            CellDistinction::Structured
        ));
        assert!(!staged_value_ok_for_distinction(
            "not-json",
            CellDistinction::Structured
        ));
    }

    #[test]
    fn boolean_toggle_and_set_null_on_edit_session() {
        let mut session = CellEditSession {
            abs_row: 0,
            column: "active".into(),
            original_text: "true".into(),
            buffer: "true".into(),
            locator: Vec::new(),
            kind: CellDistinction::Boolean,
        };
        assert!(session.toggle_boolean());
        assert_eq!(session.buffer, "false");
        assert!(session.toggle_boolean());
        assert_eq!(session.buffer, "true");
        session.set_null();
        assert_eq!(session.buffer, "null");
        let mut text = CellEditSession {
            abs_row: 0,
            column: "name".into(),
            original_text: "a".into(),
            buffer: "a".into(),
            locator: Vec::new(),
            kind: CellDistinction::Text,
        };
        assert!(!text.toggle_boolean());
        assert_eq!(text.buffer, "a");
    }
}
