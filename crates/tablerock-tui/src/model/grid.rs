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

    /// Step date portion by `delta_days` for temporal cells (keeps time suffix if present).
    pub fn step_day(&mut self, delta_days: i32) -> bool {
        if self.kind != CellDistinction::Temporal || delta_days == 0 {
            return false;
        }
        self.ensure_temporal_date_base();
        let Some((y, m, d, rest)) = split_temporal_buffer(&self.buffer) else {
            return false;
        };
        let days = days_from_civil(y, m, d).saturating_add(i64::from(delta_days));
        let (ny, nm, nd) = civil_from_days(days);
        self.buffer = join_temporal_date(ny, nm, nd, rest.as_deref());
        true
    }

    /// Step calendar month by `delta_months` (clamps day into target month).
    pub fn step_month(&mut self, delta_months: i32) -> bool {
        if self.kind != CellDistinction::Temporal || delta_months == 0 {
            return false;
        }
        self.ensure_temporal_date_base();
        let Some((y, m, d, rest)) = split_temporal_buffer(&self.buffer) else {
            return false;
        };
        let (ny, nm) = add_months(y, m, delta_months);
        let max_d = days_in_month(ny, nm);
        let nd = d.min(max_d);
        self.buffer = join_temporal_date(ny, nm, nd, rest.as_deref());
        true
    }

    /// Text month grid for the date currently in the buffer (or today).
    #[must_use]
    pub fn month_calendar_text(&self) -> Option<String> {
        if self.kind != CellDistinction::Temporal {
            return None;
        }
        let (y, m, d, _) = split_temporal_buffer(&self.buffer).or_else(|| {
            let today = local_today_iso();
            split_temporal_buffer(&today)
        })?;
        Some(format_month_calendar(y, m, d))
    }

    fn ensure_temporal_date_base(&mut self) {
        let trimmed = self.buffer.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
            self.buffer = local_today_iso();
        }
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

fn parse_ymd(s: &str) -> Option<(i32, u32, u32)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

fn split_temporal_buffer(t: &str) -> Option<(i32, u32, u32, Option<String>)> {
    let t = t.trim();
    let (date_part, rest) = if let Some((d, r)) = t.split_once('T') {
        (d, Some(format!("T{r}")))
    } else if let Some((d, r)) = t.split_once(' ') {
        (d, Some(format!(" {r}")))
    } else if t.len() >= 10 && t.as_bytes().get(4) == Some(&b'-') {
        (
            &t[..10],
            if t.len() > 10 {
                Some(t[10..].to_owned())
            } else {
                None
            },
        )
    } else {
        return None;
    };
    let (y, m, d) = parse_ymd(date_part)?;
    Some((y, m, d, rest))
}

fn join_temporal_date(y: i32, m: u32, d: u32, rest: Option<&str>) -> String {
    let new_date = format!("{y:04}-{m:02}-{d:02}");
    match rest {
        Some(r) => format!("{new_date}{r}"),
        None => new_date,
    }
}

fn add_months(y: i32, m: u32, delta: i32) -> (i32, u32) {
    let idx = i64::from(y) * 12 + i64::from(m as i32 - 1) + i64::from(delta);
    let ny = idx.div_euclid(12) as i32;
    let nm = (idx.rem_euclid(12) as u32) + 1;
    (ny, nm)
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(y) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn is_leap_year(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

/// Month grid with selected day marked `*dd`.
fn format_month_calendar(y: i32, m: u32, selected_day: u32) -> String {
    let month_names = [
        "", "January", "February", "March", "April", "May", "June", "July", "August",
        "September", "October", "November", "December",
    ];
    let name = month_names.get(m as usize).copied().unwrap_or("?");
    let mut out = format!("{name} {y}\nSu Mo Tu We Th Fr Sa\n");
    let wd1 = weekday_sunday0(y, m, 1);
    let dim = days_in_month(y, m);
    let mut line = String::new();
    for _ in 0..wd1 {
        line.push_str("   ");
    }
    let mut col = wd1;
    for day in 1..=dim {
        let cell = if day == selected_day {
            format!("*{day:2}")
        } else {
            format!(" {day:2}")
        };
        line.push_str(&cell);
        col += 1;
        if col == 7 {
            out.push_str(&line);
            out.push('\n');
            line.clear();
            col = 0;
        }
    }
    if !line.is_empty() {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// 0=Sunday .. 6=Saturday for civil date.
fn weekday_sunday0(y: i32, m: u32, d: u32) -> u32 {
    // days_from_civil epoch 1970-01-01 was Thursday; Unix day 0 weekday:
    // 1970-01-01 = Thursday = 4 if Sunday=0.
    let days = days_from_civil(y, m, d);
    // 1970-01-01 civil days = 0 relative? days_from_civil(1970,1,1):
    // Use known: days_from_civil returns days since 1970-01-01 for that function?
    // Our days_from_civil uses Howard algorithm with -719468 offset; for 1970-01-01
    // result is 0. Weekday of 1970-01-01 is Thursday (4).
    ((days + 4).rem_euclid(7)) as u32
}

/// Howard Hinnant days-from-civil (proleptic Gregorian).
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let mut y = i64::from(y);
    let m = i64::from(m);
    let d = i64::from(d);
    y -= i64::from(m <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
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
    /// Bounded ring of redacted server NOTICE/status lines for this tab.
    pub notice_history: Vec<String>,
}

/// Cap for per-tab notice history (oldest dropped first).
pub const MAX_NOTICE_HISTORY: usize = 16;

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
            notice_history: Vec::new(),
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

    /// Presentation texts for one staged insert draft across `visible` columns.
    ///
    /// Glyph `+` marks inserted rows (never color alone). Empty values show `∅`.
    #[must_use]
    pub fn insert_row_display(&self, draft_id: u64, visible: &[String]) -> Option<Vec<String>> {
        let insert = self.drafts.inserts.iter().find(|i| i.draft_id == draft_id)?;
        Some(
            visible
                .iter()
                .map(|name| {
                    let val = insert
                        .values
                        .iter()
                        .find(|(c, _)| c == name)
                        .map(|(_, v)| v.as_str())
                        .unwrap_or("");
                    if val.is_empty() {
                        "+ ∅".into()
                    } else {
                        format!("+ {val}")
                    }
                })
                .collect(),
        )
    }

    /// Presentation text for VirtualGrid paint: live edit buffer, staged
    /// overlays, and draft markers (text+glyph; never color alone).
    #[must_use]
    pub fn cell_display_at(&self, abs_row: u64, col: usize) -> String {
        use super::mutation_draft::DraftMarker;

        let col_name = self.columns.get(col).map(String::as_str).unwrap_or("");
        if let Some(edit) = self.cell_edit.as_ref() {
            if edit.abs_row == abs_row && edit.column == col_name {
                return format!("✎ {}", edit.buffer);
            }
        }
        let row_marker = self.drafts.row_marker(abs_row);
        if matches!(row_marker, DraftMarker::Deleted) {
            let base = self.cell_at(abs_row, col).display();
            return if base.is_empty() {
                DraftMarker::Deleted.glyph().into()
            } else {
                format!("{} {base}", DraftMarker::Deleted.glyph())
            };
        }
        if let Some(staged) = self.drafts.staged_for_cell(abs_row, col_name) {
            let glyph = DraftMarker::Modified.glyph();
            return if staged.is_empty() {
                format!("{glyph} ∅")
            } else {
                format!("{glyph} {staged}")
            };
        }
        let base = self.cell_at(abs_row, col).display();
        if matches!(row_marker, DraftMarker::Modified) {
            // Row has other staged cells; leave unstaged cells unmarked.
            return base;
        }
        base
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
            if self.identity_columns.is_empty() {
                String::new()
            } else {
                let keys = self.identity_columns.iter().take(4).cloned().collect::<Vec<_>>();
                let more = if self.identity_columns.len() > 4 {
                    format!("+{}", self.identity_columns.len() - 4)
                } else {
                    String::new()
                };
                format!(" · pk {}{more}", keys.join(","))
            }
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

    /// Visual multi-column sort chip bar (glyph + text, never color alone).
    ///
    /// Empty when no server sort keys. Index 0 is primary ORDER BY.
    #[must_use]
    pub fn sort_chip_bar(&self) -> Option<String> {
        if self.sort.is_empty() {
            return None;
        }
        let chips: Vec<String> = self
            .sort
            .iter()
            .take(8)
            .map(|k| format!("[{}{}]", k.column, k.direction.glyph()))
            .collect();
        let mut line = format!("sort: {}", chips.join(" "));
        if self.sort.len() > 8 {
            line.push_str(&format!(" [+{} more]", self.sort.len() - 8));
        }
        Some(line)
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

    /// Unstage the cursor cell edit only.
    pub fn unstage_cursor_cell(&mut self) -> bool {
        if self.columns.is_empty() {
            return false;
        }
        let col = self
            .columns
            .get(self.cursor_col.min(self.columns.len().saturating_sub(1)))
            .cloned()
            .unwrap_or_default();
        if col.is_empty() {
            return false;
        }
        // Cancel live edit session on this cell if open.
        if self
            .cell_edit
            .as_ref()
            .is_some_and(|e| e.abs_row == self.cursor_row && e.column == col)
        {
            self.cell_edit = None;
        }
        self.drafts.discard_cell_edit(self.cursor_row, &col)
    }

    /// Unstage all drafts for the cursor row (cell edits + delete).
    pub fn unstage_cursor_row(&mut self) -> bool {
        if self
            .cell_edit
            .as_ref()
            .is_some_and(|e| e.abs_row == self.cursor_row)
        {
            self.cell_edit = None;
        }
        self.drafts.discard_row_stages(self.cursor_row)
    }

    /// Append a redacted notice/status line; drops oldest beyond [`MAX_NOTICE_HISTORY`].
    pub fn push_notice(&mut self, summary: impl Into<String>) {
        let summary = summary.into();
        if summary.is_empty() {
            return;
        }
        self.notice_history.push(summary);
        while self.notice_history.len() > MAX_NOTICE_HISTORY {
            self.notice_history.remove(0);
        }
    }

    /// Clear notice history for this tab.
    pub fn clear_notices(&mut self) {
        self.notice_history.clear();
    }

    /// Multi-line text for the notices inspector panel (newest last).
    #[must_use]
    pub fn notices_panel_text(&self) -> String {
        if self.notice_history.is_empty() {
            return "no notices this tab".into();
        }
        let mut lines = Vec::with_capacity(self.notice_history.len() + 1);
        lines.push(format!(
            "{} notice(s) (newest last, cap {MAX_NOTICE_HISTORY})",
            self.notice_history.len()
        ));
        for (i, n) in self.notice_history.iter().enumerate() {
            lines.push(format!("{}. {n}", i + 1));
        }
        lines.join("\n")
    }

    /// Stage a blank insert (all columns empty → NULL at plan build).
    ///
    /// Generated/default columns stay empty so the server invents them on apply.
    pub fn stage_insert_blank(&mut self) -> Option<u64> {
        if !self.drafts.staging_allowed() || self.columns.is_empty() {
            return None;
        }
        let values: Vec<(String, String)> = self
            .columns
            .iter()
            .map(|c| (c.clone(), String::new()))
            .collect();
        self.drafts.stage_insert(values)
    }

    /// Stage an insert prefilled from the cursor row's presentation text.
    ///
    /// Useful as "duplicate as insert". Identity/generated columns keep the
    /// copied values; the operator may conflict on apply and must re-review.
    pub fn stage_insert_from_cursor(&mut self) -> Option<u64> {
        if !self.drafts.staging_allowed() || self.columns.is_empty() {
            return None;
        }
        let values: Vec<(String, String)> = self
            .columns
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let cell = self.cell_at(self.cursor_row, i);
                let text = if matches!(
                    cell.distinction,
                    CellDistinction::Null | CellDistinction::Pending
                ) {
                    String::new()
                } else {
                    cell.text.clone()
                };
                (name.clone(), text)
            })
            .collect();
        self.drafts.stage_insert(values)
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

    /// Append `column` as a secondary sort key (or cycle its direction in place).
    ///
    /// Unlike [`cycle_sort_column`], does not promote the key to primary. Use
    /// this to build multi-column ORDER BY lists deliberately.
    pub fn push_sort_column(&mut self, column: &str) {
        let existing = self.sort.iter().position(|k| k.column == column);
        if let Some(idx) = existing {
            let next = self.sort[idx].direction.cycle();
            if matches!(next, ColumnSort::None) {
                self.sort.remove(idx);
            } else {
                self.sort[idx].direction = next;
            }
        } else {
            self.sort.push(GridSortKey {
                column: column.to_owned(),
                direction: ColumnSort::Asc,
            });
        }
    }

    /// Remove the least-significant (last) sort key. Returns true if one removed.
    pub fn pop_sort_key(&mut self) -> bool {
        self.sort.pop().is_some()
    }

    /// Flip primary sort direction Asc↔Desc. Returns false if no sort.
    pub fn invert_primary_sort(&mut self) -> bool {
        let Some(key) = self.sort.first_mut() else {
            return false;
        };
        key.direction = match key.direction {
            ColumnSort::Asc => ColumnSort::Desc,
            ColumnSort::Desc => ColumnSort::Asc,
            ColumnSort::None => ColumnSort::Asc,
        };
        true
    }

    /// Flip every sort key Asc↔Desc. Returns false if no sort.
    pub fn invert_all_sorts(&mut self) -> bool {
        if self.sort.is_empty() {
            return false;
        }
        for key in &mut self.sort {
            key.direction = match key.direction {
                ColumnSort::Asc => ColumnSort::Desc,
                ColumnSort::Desc => ColumnSort::Asc,
                ColumnSort::None => ColumnSort::Asc,
            };
        }
        true
    }

    /// Rotate sort keys left: secondary becomes primary. Needs ≥2 keys.
    pub fn rotate_sort_keys(&mut self) -> bool {
        if self.sort.len() < 2 {
            return false;
        }
        let first = self.sort.remove(0);
        self.sort.push(first);
        true
    }

    /// Rotate sort keys right: last key becomes primary. Needs ≥2 keys.
    pub fn rotate_sort_keys_right(&mut self) -> bool {
        if self.sort.len() < 2 {
            return false;
        }
        let last = self.sort.pop().expect("len checked");
        self.sort.insert(0, last);
        true
    }

    /// Drop secondary sort keys; keep only the primary. Needs ≥2 keys.
    pub fn keep_primary_sort(&mut self) -> bool {
        if self.sort.len() < 2 {
            return false;
        }
        self.sort.truncate(1);
        true
    }

    /// Swap primary and secondary sort keys. Needs ≥2 keys.
    pub fn swap_primary_secondary_sort(&mut self) -> bool {
        if self.sort.len() < 2 {
            return false;
        }
        self.sort.swap(0, 1);
        true
    }

    /// Reverse the entire multi-key sort list. Needs ≥2 keys.
    pub fn reverse_sort_keys(&mut self) -> bool {
        if self.sort.len() < 2 {
            return false;
        }
        self.sort.reverse();
        true
    }

    /// Promote `column` to primary ORDER BY without cycling direction.
    ///
    /// - Already primary: no-op (false).
    /// - Present as secondary: move to index 0, keep direction.
    /// - Absent: insert Asc as new primary (secondaries stay).
    pub fn promote_sort_column(&mut self, column: &str) -> bool {
        if let Some(idx) = self.sort.iter().position(|k| k.column == column) {
            if idx == 0 {
                return false;
            }
            let key = self.sort.remove(idx);
            self.sort.insert(0, key);
            return true;
        }
        self.sort.insert(
            0,
            GridSortKey {
                column: column.to_owned(),
                direction: ColumnSort::Asc,
            },
        );
        true
    }

    /// Clear server sort/filter; keep quick filter (page-local).
    pub fn clear_server_controls(&mut self) {
        self.sort.clear();
        self.filters.clear();
        self.raw_where = None;
    }

    /// Clear sort keys only (keep filters and raw WHERE).
    pub fn clear_sort(&mut self) -> bool {
        if self.sort.is_empty() {
            return false;
        }
        self.sort.clear();
        true
    }

    /// Clear typed filters + raw WHERE only (keep sort keys).
    pub fn clear_filters_keep_sort(&mut self) -> bool {
        let had_filters = !self.filters.is_empty();
        let had_raw = self.raw_where.as_ref().is_some_and(|s| !s.is_empty());
        if !had_filters && !had_raw {
            return false;
        }
        self.filters.clear();
        self.raw_where = None;
        true
    }

    /// Remove the most recently added server filter chip. Returns true if one was removed.
    pub fn remove_last_filter(&mut self) -> bool {
        self.filters.pop().is_some()
    }

    /// Remove the oldest server filter chip. Returns true if one was removed.
    pub fn remove_first_filter(&mut self) -> bool {
        if self.filters.is_empty() {
            return false;
        }
        self.filters.remove(0);
        true
    }

    /// Reverse server filter chip order (AND order). Needs ≥2 chips.
    pub fn reverse_filters(&mut self) -> bool {
        if self.filters.len() < 2 {
            return false;
        }
        self.filters.reverse();
        true
    }

    /// Move the newest filter chip to the front (AND primary). Needs ≥2 chips.
    pub fn promote_last_filter(&mut self) -> bool {
        if self.filters.len() < 2 {
            return false;
        }
        let last = self.filters.pop().expect("len checked");
        self.filters.insert(0, last);
        true
    }

    /// Remove all server filters for a column name (keeps sort/raw_where).
    pub fn remove_filters_for_column(&mut self, column: &str) -> usize {
        let before = self.filters.len();
        self.filters.retain(|f| f.column != column);
        before.saturating_sub(self.filters.len())
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

    /// Reset every layout width to the default (12); keep order and visibility.
    ///
    /// Returns false when nothing changed (already all default widths).
    pub fn reset_column_widths(&mut self) -> bool {
        self.ensure_column_layout();
        let mut changed = false;
        for entry in &mut self.column_layout {
            if entry.width != 12 {
                entry.width = 12;
                changed = true;
            }
        }
        changed
    }

    /// Set every visible column width to the cursor column's width (4..=64).
    pub fn equalize_visible_column_widths(&mut self) -> bool {
        self.ensure_column_layout();
        let Some(name) = self.columns.get(self.cursor_col).cloned() else {
            return false;
        };
        let Some(width) = self
            .column_layout
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.width)
        else {
            return false;
        };
        let mut changed = false;
        for entry in &mut self.column_layout {
            if entry.visible && entry.width != width {
                entry.width = width;
                changed = true;
            }
        }
        changed
    }

    /// Hide all columns except the cursor column. Returns true if layout changed.
    pub fn solo_cursor_column(&mut self) -> bool {
        if self.columns.is_empty() {
            return false;
        }
        self.ensure_column_layout();
        let name = self
            .columns
            .get(self.cursor_col.min(self.columns.len().saturating_sub(1)))
            .cloned()
            .unwrap_or_default();
        if name.is_empty() {
            return false;
        }
        let mut changed = false;
        for entry in &mut self.column_layout {
            let want = entry.name == name;
            if entry.visible != want {
                entry.visible = want;
                changed = true;
            }
        }
        changed
    }

    /// Show only identity (pk) columns; keep widths/order. Needs known identity.
    pub fn solo_identity_columns(&mut self) -> bool {
        if self.identity_columns.is_empty() || self.columns.is_empty() {
            return false;
        }
        self.ensure_column_layout();
        let mut changed = false;
        for entry in &mut self.column_layout {
            let want = self.identity_columns.iter().any(|id| id == &entry.name);
            if entry.visible != want {
                entry.visible = want;
                changed = true;
            }
        }
        // Fail closed: if no identity column matched layout, restore first identity if present.
        if !self.column_layout.iter().any(|c| c.visible) {
            if let Some(entry) = self
                .column_layout
                .iter_mut()
                .find(|c| self.identity_columns.iter().any(|id| id == &c.name))
            {
                entry.visible = true;
                changed = true;
            } else if let Some(first) = self.column_layout.first_mut() {
                first.visible = true;
                changed = true;
            }
        }
        changed
    }

    /// Hide columns whose every resident cell is Null or empty text.
    ///
    /// Page-local only (no I/O). Keeps at least one column visible. Returns
    /// true if any column was hidden.
    pub fn hide_empty_resident_columns(&mut self) -> bool {
        if self.columns.is_empty() || self.row_count == 0 {
            return false;
        }
        self.ensure_column_layout();
        let mut empty_names = Vec::new();
        for (ci, name) in self.columns.iter().enumerate() {
            let mut all_empty = true;
            for local in 0..self.row_count {
                let abs = self.start_row.saturating_add(u64::from(local));
                let cell = self.cell_at(abs, ci);
                let empty = matches!(
                    cell.distinction,
                    CellDistinction::Null | CellDistinction::Empty | CellDistinction::Pending
                ) || cell.text.is_empty();
                if !empty {
                    all_empty = false;
                    break;
                }
            }
            if all_empty {
                empty_names.push(name.clone());
            }
        }
        if empty_names.is_empty() {
            return false;
        }
        let mut changed = false;
        for entry in &mut self.column_layout {
            if empty_names.iter().any(|n| n == &entry.name) && entry.visible {
                entry.visible = false;
                changed = true;
            }
        }
        if !self.column_layout.iter().any(|c| c.visible) {
            // Fail closed: restore first layout entry.
            if let Some(first) = self.column_layout.first_mut() {
                first.visible = true;
            }
        }
        changed
    }

    /// Show every column; keep widths and order. Returns true if any were hidden.
    pub fn show_all_columns(&mut self) -> bool {
        if self.columns.is_empty() {
            return false;
        }
        self.ensure_column_layout();
        let mut changed = false;
        for entry in &mut self.column_layout {
            if !entry.visible {
                entry.visible = true;
                changed = true;
            }
        }
        changed
    }

    /// Invert visibility of every column. Guarantees at least one stays visible
    /// (if invert would hide all, leaves the first layout entry visible).
    pub fn invert_column_visibility(&mut self) -> bool {
        if self.columns.is_empty() {
            return false;
        }
        self.ensure_column_layout();
        if self.column_layout.is_empty() {
            return false;
        }
        for entry in &mut self.column_layout {
            entry.visible = !entry.visible;
        }
        if !self.column_layout.iter().any(|c| c.visible) {
            self.column_layout[0].visible = true;
        }
        true
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
    /// Presentation of identity locator for the cursor row (`col=value` lines).
    #[must_use]
    pub fn cursor_locator_text(&self) -> Option<String> {
        if self.identity_columns.is_empty() {
            return None;
        }
        let fields = self.locator_for_row(self.cursor_row);
        if fields.is_empty() {
            return None;
        }
        Some(
            fields
                .iter()
                .map(|f| format!("{}={}", f.column, f.original_text))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    /// SQL WHERE fragment from cursor row locator (quoted idents, literal values).
    ///
    /// Presentation aid only — not executed. Values are single-quoted with `'`
    /// doubled. NULL cells use the `NULL` keyword.
    #[must_use]
    pub fn cursor_where_sql(&self) -> Option<String> {
        let fields = self.locator_for_row(self.cursor_row);
        if fields.is_empty() {
            return None;
        }
        let parts: Vec<String> = fields
            .iter()
            .map(|f| {
                let col = format!("\"{}\"", f.column.replace('"', "\"\""));
                let is_null = self
                    .columns
                    .iter()
                    .position(|c| c == &f.column)
                    .map(|i| {
                        matches!(
                            self.cell_at(self.cursor_row, i).distinction,
                            CellDistinction::Null
                        )
                    })
                    .unwrap_or(false);
                let lit = if is_null {
                    "NULL".into()
                } else {
                    format!("'{}'", f.original_text.replace('\'', "''"))
                };
                format!("{col} = {lit}")
            })
            .collect();
        Some(format!("WHERE {}", parts.join(" AND ")))
    }

    /// Align horizontal viewport so the cursor column is the first visible
    /// physical column when possible (wide grids after GoToColumn).
    pub fn reveal_cursor_column(&mut self) {
        if self.columns.is_empty() {
            return;
        }
        let col = self.cursor_col.min(self.columns.len().saturating_sub(1));
        self.cursor_col = col;
        self.viewport_col = col;
    }

    /// Jump cursor to the first identity (pk) column in physical order.
    ///
    /// Returns false when identity is unknown or no identity column is present.
    pub fn go_to_first_identity_column(&mut self) -> bool {
        if self.identity_columns.is_empty() || self.columns.is_empty() {
            return false;
        }
        for name in &self.identity_columns {
            if let Some(idx) = self.columns.iter().position(|c| c == name) {
                if self.cursor_col == idx {
                    self.reveal_cursor_column();
                    return false; // already there
                }
                self.cursor_col = idx;
                self.reveal_cursor_column();
                return true;
            }
        }
        false
    }

    /// Jump cursor to the last identity (pk) column in identity-list order.
    pub fn go_to_last_identity_column(&mut self) -> bool {
        if self.identity_columns.is_empty() || self.columns.is_empty() {
            return false;
        }
        for name in self.identity_columns.iter().rev() {
            if let Some(idx) = self.columns.iter().position(|c| c == name) {
                if self.cursor_col == idx {
                    self.reveal_cursor_column();
                    return false;
                }
                self.cursor_col = idx;
                self.reveal_cursor_column();
                return true;
            }
        }
        false
    }

    /// Move cursor to the first resident cell (no server I/O).
    pub fn home_cursor(&mut self) {
        self.cursor_row = self.start_row;
        self.cursor_col = 0;
        self.viewport_row = self.start_row;
        self.viewport_col = 0;
    }

    /// Move cursor to the last resident cell (no server I/O).
    pub fn end_cursor(&mut self) {
        if self.columns.is_empty() || self.row_count == 0 {
            self.home_cursor();
            return;
        }
        self.cursor_row = self
            .start_row
            .saturating_add(u64::from(self.row_count.saturating_sub(1)));
        self.cursor_col = self.columns.len().saturating_sub(1);
        self.viewport_row = self.cursor_row;
        self.viewport_col = self.cursor_col;
    }

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

    /// Move cursor column to first (edge < 0) or last (edge > 0) layout slot.
    pub fn move_cursor_column_to_edge(&mut self, edge: i8) -> bool {
        if edge == 0 {
            return false;
        }
        self.ensure_column_layout();
        let Some(name) = self.columns.get(self.cursor_col).cloned() else {
            return false;
        };
        let Some(idx) = self.column_layout.iter().position(|c| c.name == name) else {
            return false;
        };
        let target = if edge < 0 {
            0
        } else {
            self.column_layout.len().saturating_sub(1)
        };
        if idx == target {
            return false;
        }
        let entry = self.column_layout.remove(idx);
        self.column_layout.insert(target, entry);
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

    /// Jump cursor and viewport to absolute row (clamped to known totals when Exact).
    ///
    /// Returns the target row after clamp, or `None` if empty grid / invalid.
    pub fn go_to_row(&mut self, target: u64) -> Option<u64> {
        if self.columns.is_empty() {
            return None;
        }
        let max = match self.totals {
            GridRowTotal::Exact(n) if n > 0 => n.saturating_sub(1),
            GridRowTotal::Estimated(n) if n > 0 => n.saturating_sub(1),
            _ => {
                // Unknown totals: allow jump within resident or unbounded up to 1e9-1.
                u64::MAX / 4
            }
        };
        let row = target.min(max);
        self.cursor_row = row;
        self.viewport_row = row;
        Some(row)
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
    fn home_cursor_resets_viewport() {
        let mut g = DataGridModel::default();
        g.columns = vec!["a".into(), "b".into()];
        g.start_row = 100;
        g.row_count = 10;
        g.cursor_row = 107;
        g.cursor_col = 1;
        g.viewport_row = 105;
        g.viewport_col = 1;
        g.home_cursor();
        assert_eq!(g.cursor_row, 100);
        assert_eq!(g.cursor_col, 0);
        assert_eq!(g.viewport_row, 100);
        assert_eq!(g.viewport_col, 0);
        g.end_cursor();
        assert_eq!(g.cursor_row, 109);
        assert_eq!(g.cursor_col, 1);
        assert_eq!(g.viewport_row, 109);
        assert_eq!(g.viewport_col, 1);
    }

    #[test]
    fn cursor_locator_text_for_identity() {
        let mut g = DataGridModel::default();
        g.columns = vec!["id".into(), "name".into()];
        g.row_count = 1;
        g.cells = vec![
            ProjectedCell {
                text: "7".into(),
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
        ];
        g.identity_columns = vec!["id".into()];
        g.cursor_row = 0;
        let loc = g.cursor_locator_text().expect("locator");
        assert_eq!(loc, "id=7");
        let wh = g.cursor_where_sql().expect("where");
        assert_eq!(wh, "WHERE \"id\" = '7'");
        g.identity_columns.clear();
        assert!(g.cursor_locator_text().is_none());
        assert!(g.cursor_where_sql().is_none());
    }

    #[test]
    fn reveal_cursor_column_sets_viewport() {
        let mut g = DataGridModel::default();
        g.columns = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        g.cursor_col = 3;
        g.viewport_col = 0;
        g.reveal_cursor_column();
        assert_eq!(g.cursor_col, 3);
        assert_eq!(g.viewport_col, 3);
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
    fn solo_cursor_column_hides_others() {
        let mut g = DataGridModel::default();
        g.columns = vec!["id".into(), "name".into(), "age".into()];
        g.cursor_col = 1; // name
        assert!(g.solo_cursor_column());
        assert_eq!(g.visible_columns(), vec!["name".to_owned()]);
        // Already solo → no-op.
        assert!(!g.solo_cursor_column());
        // Invert: name hidden, id+age shown.
        assert!(g.invert_column_visibility());
        let vis = g.visible_columns();
        assert!(vis.contains(&"id".to_owned()));
        assert!(vis.contains(&"age".to_owned()));
        assert!(!vis.contains(&"name".to_owned()));
        // Widen then ShowAll keeps width.
        g.cursor_col = 0;
        assert!(g.adjust_cursor_column_width(8));
        let w = g.column_width("id");
        assert!(g.show_all_columns());
        assert_eq!(g.visible_columns().len(), 3);
        assert_eq!(g.column_width("id"), w);
        assert!(!g.show_all_columns());
        g.reset_column_layout();
        assert_eq!(g.visible_columns().len(), 3);
    }

    #[test]
    fn solo_identity_columns_shows_pk_only() {
        let mut g = DataGridModel::default();
        g.columns = vec!["id".into(), "tenant_id".into(), "name".into(), "age".into()];
        assert!(!g.solo_identity_columns()); // no identity yet
        g.identity_columns = vec!["tenant_id".into(), "id".into()];
        assert!(g.solo_identity_columns());
        let vis = g.visible_columns();
        assert_eq!(vis.len(), 2);
        assert!(vis.contains(&"id".to_owned()));
        assert!(vis.contains(&"tenant_id".to_owned()));
        assert!(!vis.contains(&"name".to_owned()));
        assert!(!g.solo_identity_columns()); // already
    }

    #[test]
    fn go_to_first_identity_column_jumps() {
        let mut g = DataGridModel::default();
        g.columns = vec!["name".into(), "tenant_id".into(), "id".into()];
        g.cursor_col = 0;
        assert!(!g.go_to_first_identity_column());
        g.identity_columns = vec!["tenant_id".into(), "id".into()];
        assert!(g.go_to_first_identity_column());
        assert_eq!(g.cursor_col, 1); // tenant_id first in identity list
        assert!(!g.go_to_first_identity_column());
        g.cursor_col = 2;
        assert!(g.go_to_first_identity_column());
        assert_eq!(g.cursor_col, 1);
        assert!(g.go_to_last_identity_column());
        assert_eq!(g.cursor_col, 2); // id last in identity list
        assert!(!g.go_to_last_identity_column());
    }

    #[test]
    fn hide_empty_resident_columns_keeps_one() {
        let mut g = DataGridModel::default();
        g.columns = vec!["id".into(), "note".into(), "flag".into()];
        g.row_count = 2;
        g.start_row = 0;
        g.cells = vec![
            ProjectedCell {
                text: "1".into(),
                distinction: CellDistinction::Number,
                byte_len: 1,
                original_byte_len: None,
            },
            ProjectedCell {
                text: String::new(),
                distinction: CellDistinction::Null,
                byte_len: 0,
                original_byte_len: None,
            },
            ProjectedCell {
                text: String::new(),
                distinction: CellDistinction::Empty,
                byte_len: 0,
                original_byte_len: None,
            },
            ProjectedCell {
                text: "2".into(),
                distinction: CellDistinction::Number,
                byte_len: 1,
                original_byte_len: None,
            },
            ProjectedCell {
                text: String::new(),
                distinction: CellDistinction::Null,
                byte_len: 0,
                original_byte_len: None,
            },
            ProjectedCell {
                text: String::new(),
                distinction: CellDistinction::Empty,
                byte_len: 0,
                original_byte_len: None,
            },
        ];
        assert!(g.hide_empty_resident_columns());
        let vis = g.visible_columns();
        assert_eq!(vis, vec!["id".to_owned()]);
        assert!(!g.hide_empty_resident_columns());
    }

    #[test]
    fn equalize_visible_column_widths_uses_cursor() {
        let mut g = DataGridModel::default();
        g.columns = vec!["id".into(), "name".into(), "age".into()];
        g.cursor_col = 0;
        assert!(g.adjust_cursor_column_width(8)); // id -> 20
        assert_eq!(g.column_width("id"), 20);
        assert_eq!(g.column_width("name"), 12);
        assert!(g.equalize_visible_column_widths());
        assert_eq!(g.column_width("id"), 20);
        assert_eq!(g.column_width("name"), 20);
        assert_eq!(g.column_width("age"), 20);
        assert!(!g.equalize_visible_column_widths());
        // Hidden columns keep their width.
        g.cursor_col = 1;
        assert!(g.solo_cursor_column()); // only name visible
        g.cursor_col = 1;
        assert!(g.adjust_cursor_column_width(-4)); // name 16
        // Make id visible again without equalizing yet.
        assert!(g.show_all_columns());
        // name is 16; id and age still 20 from earlier equalize + show all.
        g.cursor_col = 1;
        assert!(g.equalize_visible_column_widths());
        assert_eq!(g.column_width("name"), 16);
        assert_eq!(g.column_width("id"), 16);
        assert_eq!(g.column_width("age"), 16);
    }

    #[test]
    fn move_cursor_column_to_edge_jumps_layout() {
        let mut g = DataGridModel::default();
        g.columns = vec!["id".into(), "name".into(), "age".into()];
        g.cursor_col = 1; // name
        assert!(g.move_cursor_column_to_edge(1)); // to last
        assert_eq!(
            g.column_layout.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            vec!["id", "age", "name"]
        );
        assert!(!g.move_cursor_column_to_edge(1)); // already last
        assert!(g.move_cursor_column_to_edge(-1)); // to first
        assert_eq!(
            g.column_layout.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            vec!["name", "id", "age"]
        );
        assert!(!g.move_cursor_column_to_edge(-1));
    }

    #[test]
    fn reset_column_widths_keeps_order_and_visibility() {
        let mut g = DataGridModel::default();
        g.columns = vec!["id".into(), "name".into(), "age".into()];
        g.cursor_col = 1;
        assert!(g.solo_cursor_column());
        assert!(g.adjust_cursor_column_width(20));
        assert_eq!(g.column_width("name"), 32); // 12+20
        assert!(g.reset_column_widths());
        assert_eq!(g.column_width("name"), 12);
        assert_eq!(g.visible_columns(), vec!["name".to_owned()]);
        assert!(!g.reset_column_widths());
        // Order preserved after prior move.
        g.show_all_columns();
        g.cursor_col = 0;
        assert!(g.move_cursor_column(1));
        assert!(g.adjust_cursor_column_width(4));
        assert!(g.reset_column_widths());
        assert_eq!(
            g.column_layout
                .iter()
                .map(|c| (c.name.as_str(), c.width, c.visible))
                .collect::<Vec<_>>(),
            vec![("name", 12, true), ("id", 12, true), ("age", 12, true)]
        );
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
    fn invert_primary_sort_flips_direction() {
        let mut g = DataGridModel::default();
        assert!(!g.invert_primary_sort());
        g.push_sort_column("name");
        assert_eq!(g.sort[0].direction, ColumnSort::Asc);
        assert!(g.invert_primary_sort());
        assert_eq!(g.sort[0].direction, ColumnSort::Desc);
        assert!(g.invert_primary_sort());
        assert_eq!(g.sort[0].direction, ColumnSort::Asc);
        assert!(!g.rotate_sort_keys());
        g.push_sort_column("age");
        assert_eq!(g.sort[0].column, "name");
        assert_eq!(g.sort[1].column, "age");
        assert!(g.rotate_sort_keys());
        assert_eq!(g.sort[0].column, "age");
        assert_eq!(g.sort[1].column, "name");
        // Right-rotate undoes a left-rotate on two keys.
        assert!(g.rotate_sort_keys_right());
        assert_eq!(g.sort[0].column, "name");
        assert_eq!(g.sort[1].column, "age");
        g.push_sort_column("id");
        assert_eq!(g.sort.len(), 3);
        assert!(g.rotate_sort_keys_right());
        assert_eq!(g.sort[0].column, "id");
        assert_eq!(g.sort[1].column, "name");
        assert_eq!(g.sort[2].column, "age");
        assert!(g.keep_primary_sort());
        assert_eq!(g.sort.len(), 1);
        assert_eq!(g.sort[0].column, "id");
        assert!(!g.keep_primary_sort());
        assert!(!g.rotate_sort_keys_right());
    }

    #[test]
    fn promote_sort_column_moves_without_cycle() {
        let mut g = DataGridModel::default();
        g.columns = vec!["id".into(), "name".into(), "age".into()];
        assert!(g.promote_sort_column("name"));
        assert_eq!(g.sort.len(), 1);
        assert_eq!(g.sort[0].column, "name");
        assert_eq!(g.sort[0].direction, ColumnSort::Asc);
        // Already primary — no-op.
        assert!(!g.promote_sort_column("name"));
        g.push_sort_column("age");
        g.push_sort_column("id");
        // Flip secondary age to Desc in place.
        g.push_sort_column("age");
        assert_eq!(g.sort[1].direction, ColumnSort::Desc);
        assert!(g.promote_sort_column("age"));
        assert_eq!(g.sort[0].column, "age");
        assert_eq!(g.sort[0].direction, ColumnSort::Desc);
        assert_eq!(g.sort[1].column, "name");
        assert_eq!(g.sort[2].column, "id");
    }

    #[test]
    fn swap_primary_secondary_sort_exchanges_first_two() {
        let mut g = DataGridModel::default();
        assert!(!g.swap_primary_secondary_sort());
        g.push_sort_column("name");
        assert!(!g.swap_primary_secondary_sort());
        g.push_sort_column("age");
        g.push_sort_column("id");
        assert_eq!(g.sort[0].column, "name");
        assert_eq!(g.sort[1].column, "age");
        assert_eq!(g.sort[2].column, "id");
        assert!(g.swap_primary_secondary_sort());
        assert_eq!(g.sort[0].column, "age");
        assert_eq!(g.sort[1].column, "name");
        assert_eq!(g.sort[2].column, "id"); // tertiary untouched
    }

    #[test]
    fn reverse_sort_keys_full_list() {
        let mut g = DataGridModel::default();
        assert!(!g.reverse_sort_keys());
        g.push_sort_column("a");
        assert!(!g.reverse_sort_keys());
        g.push_sort_column("b");
        g.push_sort_column("c");
        assert!(g.reverse_sort_keys());
        assert_eq!(
            g.sort.iter().map(|k| k.column.as_str()).collect::<Vec<_>>(),
            vec!["c", "b", "a"]
        );
        assert!(g.reverse_sort_keys());
        assert_eq!(g.sort[0].column, "a");
    }

    #[test]
    fn invert_all_sorts_flips_every_key() {
        let mut g = DataGridModel::default();
        assert!(!g.invert_all_sorts());
        g.push_sort_column("name"); // Asc
        g.push_sort_column("age"); // Asc
        g.push_sort_column("age"); // -> Desc secondary
        assert_eq!(g.sort[0].direction, ColumnSort::Asc);
        assert_eq!(g.sort[1].direction, ColumnSort::Desc);
        assert!(g.invert_all_sorts());
        assert_eq!(g.sort[0].direction, ColumnSort::Desc);
        assert_eq!(g.sort[1].direction, ColumnSort::Asc);
        assert_eq!(g.sort[0].column, "name");
        assert_eq!(g.sort[1].column, "age");
    }

    #[test]
    fn push_sort_builds_multi_column_and_chip_bar() {
        let mut g = DataGridModel::default();
        g.columns = vec!["id".into(), "name".into(), "age".into()];
        g.push_sort_column("name");
        g.push_sort_column("age");
        assert_eq!(g.sort.len(), 2);
        assert_eq!(g.sort[0].column, "name");
        assert_eq!(g.sort[0].direction, ColumnSort::Asc);
        assert_eq!(g.sort[1].column, "age");
        // Cycle secondary in place — still secondary.
        g.push_sort_column("age");
        assert_eq!(g.sort[0].column, "name");
        assert_eq!(g.sort[1].direction, ColumnSort::Desc);
        let bar = g.sort_chip_bar().expect("sort bar");
        assert!(bar.contains("[name↑]"), "{bar}");
        assert!(bar.contains("[age↓]"), "{bar}");
        assert!(g.pop_sort_key());
        assert_eq!(g.sort.len(), 1);
        assert_eq!(g.sort[0].column, "name");
        // CycleSort promotes: push age again then cycle name to primary.
        g.push_sort_column("age");
        g.cycle_sort_column("name");
        assert_eq!(g.sort[0].column, "name");
        assert_eq!(g.sort[0].direction, ColumnSort::Desc);
        assert!(g.pop_sort_key());
        assert!(g.pop_sort_key());
        assert!(!g.pop_sort_key());
        assert!(g.sort_chip_bar().is_none());
    }

    #[test]
    fn clear_filters_keep_sort_preserves_order_by() {
        let mut g = DataGridModel::default();
        g.push_sort_column("name");
        g.add_filter_chip("status", "eq", Some("open".into()));
        g.raw_where = Some("deleted_at IS NULL".into());
        assert!(g.clear_filters_keep_sort());
        assert!(g.filters.is_empty());
        assert!(g.raw_where.is_none());
        assert_eq!(g.sort.len(), 1);
        assert_eq!(g.sort[0].column, "name");
        assert!(!g.clear_filters_keep_sort());
        // ClearFilters-style full clear still available separately.
        g.add_filter_chip("a", "eq", Some("1".into()));
        g.clear_server_controls();
        assert!(g.sort.is_empty());
        assert!(g.filters.is_empty());
    }

    #[test]
    fn remove_last_and_column_filters() {
        let mut g = DataGridModel::default();
        g.add_filter_chip("a", "eq", Some("1".into()));
        g.add_filter_chip("b", "isnull", None);
        g.add_filter_chip("a", "isnotnull", None);
        assert!(g.remove_last_filter());
        assert_eq!(g.filters.len(), 2);
        assert_eq!(g.remove_filters_for_column("a"), 1);
        assert_eq!(g.filters.len(), 1);
        assert_eq!(g.filters[0].column, "b");
        assert!(g.remove_last_filter());
        assert!(g.filters.is_empty());
        assert!(!g.remove_last_filter());
        g.add_filter_chip("x", "eq", Some("1".into()));
        g.add_filter_chip("y", "eq", Some("2".into()));
        g.add_filter_chip("z", "eq", Some("3".into()));
        assert!(g.remove_first_filter());
        assert_eq!(g.filters.len(), 2);
        assert_eq!(g.filters[0].column, "y");
        assert_eq!(g.filters[1].column, "z");
        assert!(g.remove_first_filter());
        assert_eq!(g.filters[0].column, "z");
        assert!(g.remove_first_filter());
        assert!(!g.remove_first_filter());
        g.add_filter_chip("a", "eq", Some("1".into()));
        g.add_filter_chip("b", "eq", Some("2".into()));
        g.add_filter_chip("c", "eq", Some("3".into()));
        assert!(g.reverse_filters());
        assert_eq!(
            g.filters
                .iter()
                .map(|f| f.column.as_str())
                .collect::<Vec<_>>(),
            vec!["c", "b", "a"]
        );
        assert!(g.reverse_filters());
        assert_eq!(g.filters[0].column, "a");
        g.filters.clear();
        g.add_filter_chip("solo", "eq", Some("1".into()));
        assert!(!g.reverse_filters());
        g.add_filter_chip("a", "eq", Some("1".into()));
        g.add_filter_chip("b", "eq", Some("2".into()));
        g.add_filter_chip("c", "eq", Some("3".into()));
        assert!(g.promote_last_filter());
        assert_eq!(
            g.filters
                .iter()
                .map(|f| f.column.as_str())
                .collect::<Vec<_>>(),
            vec!["c", "a", "b"]
        );
        assert!(g.promote_last_filter());
        assert_eq!(g.filters[0].column, "b");
        assert_eq!(g.filters[1].column, "c");
        assert_eq!(g.filters[2].column, "a");
        g.filters.clear();
        g.add_filter_chip("solo", "eq", Some("1".into()));
        assert!(!g.promote_last_filter());
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
        assert!(
            grid.status_line().contains("pk id"),
            "{}",
            grid.status_line()
        );
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
    fn notice_history_ring_and_panel_text() {
        let mut g = DataGridModel::default();
        assert_eq!(g.notices_panel_text(), "no notices this tab");
        g.push_notice("NOTICE: first");
        g.push_notice("");
        assert_eq!(g.notice_history.len(), 1);
        for i in 0..20 {
            g.push_notice(format!("NOTICE: n{i}"));
        }
        assert_eq!(g.notice_history.len(), MAX_NOTICE_HISTORY);
        assert!(!g.notice_history.iter().any(|n| n == "NOTICE: first"));
        assert!(g.notice_history.last().unwrap().contains("n19"));
        let panel = g.notices_panel_text();
        assert!(panel.contains("16 notice(s)"), "{panel}");
        assert!(panel.contains("1. "), "{panel}");
        g.clear_notices();
        assert!(g.notice_history.is_empty());
    }

    #[test]
    fn unstage_cursor_cell_and_row() {
        use super::super::mutation_draft::{DraftLocatorField, StagedCellEdit};
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
        grid.recompute_editability(ProfileSafetyMode::ConfirmWrites, false);
        grid.cursor_row = 0;
        grid.cursor_col = 1;
        assert!(grid.drafts.stage_cell_edit(StagedCellEdit {
            abs_row: 0,
            column: "name".into(),
            original_text: "alice".into(),
            staged_text: "bob".into(),
            locator: vec![DraftLocatorField {
                column: "id".into(),
                original_text: "1".into(),
            }],
        }));
        assert!(grid.unstage_cursor_cell());
        assert!(grid.drafts.cell_edits.is_empty());
        assert!(!grid.unstage_cursor_cell());
        assert!(grid.stage_delete_cursor_row());
        assert!(grid.unstage_cursor_row());
        assert!(grid.drafts.deletes.is_empty());
    }

    #[test]
    fn insert_row_display_marks_plus_glyph() {
        use tablerock_core::ProfileSafetyMode;

        let mut grid = DataGridModel::default();
        grid.columns = vec!["id".into(), "name".into()];
        grid.base_schema = Some("public".into());
        grid.base_table = Some("users".into());
        grid.identity_columns = vec!["id".into()];
        grid.recompute_editability(ProfileSafetyMode::ConfirmWrites, false);
        let id = grid.stage_insert_blank().unwrap();
        let vis = vec!["id".into(), "name".into()];
        let texts = grid.insert_row_display(id, &vis).unwrap();
        assert_eq!(texts, vec!["+ ∅".to_owned(), "+ ∅".to_owned()]);
        assert!(grid.drafts.replace_insert_values(
            id,
            vec![("id".into(), "1".into()), ("name".into(), "ada".into())]
        ));
        let texts = grid.insert_row_display(id, &vis).unwrap();
        assert_eq!(texts[0], "+ 1");
        assert_eq!(texts[1], "+ ada");
        assert!(grid.insert_row_display(99, &vis).is_none());
    }

    #[test]
    fn stage_insert_blank_and_from_cursor() {
        use tablerock_core::ProfileSafetyMode;

        let mut grid = DataGridModel::default();
        grid.columns = vec!["id".into(), "name".into()];
        grid.row_count = 1;
        grid.cells = vec![
            ProjectedCell {
                text: "7".into(),
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
        grid.recompute_editability(ProfileSafetyMode::ConfirmWrites, false);
        let blank = grid.stage_insert_blank().expect("blank");
        assert_eq!(blank, 0);
        assert_eq!(grid.drafts.inserts.len(), 1);
        assert_eq!(grid.drafts.inserts[0].values[0], ("id".into(), String::new()));
        assert_eq!(grid.drafts.inserts[0].values[1], ("name".into(), String::new()));
        let dup = grid.stage_insert_from_cursor().expect("dup");
        assert_eq!(dup, 1);
        assert_eq!(grid.drafts.inserts[1].values[0].1, "7");
        assert_eq!(grid.drafts.inserts[1].values[1].1, "alice");
        assert!(grid.status_line().contains("staged 2"));
        assert!(grid.status_line().contains("2↑"));
        // Read-only blocks.
        grid.recompute_editability(ProfileSafetyMode::ReadOnly, false);
        assert!(grid.stage_insert_blank().is_none());
        assert!(grid.stage_insert_from_cursor().is_none());
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
        assert!(
            grid.cell_display_at(0, 1).starts_with('·'),
            "staged cell needs modified glyph"
        );
        assert!(grid.cell_display_at(0, 1).contains("bob"));
        // Unstaged column on modified row stays original.
        assert_eq!(grid.cell_display_at(0, 0), "1");
        assert!(grid.stage_delete_cursor_row());
        assert!(
            grid.cell_display_at(0, 0).starts_with('−')
                || grid.cell_display_at(0, 0).starts_with('-'),
            "deleted row marker: {}",
            grid.cell_display_at(0, 0)
        );
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
    fn million_row_totals_viewport_stays_resident_window() {
        // Synthetic large total must not allocate cells for every row.
        let mut grid = DataGridModel::default();
        let page_rows = 100u32;
        let mut cells = Vec::with_capacity(page_rows as usize);
        for i in 0..page_rows {
            cells.push(ProjectedCell {
                text: i.to_string(),
                distinction: CellDistinction::Number,
                byte_len: 1,
                original_byte_len: None,
            });
        }
        grid.replace_page(
            0,
            vec!["id".into()],
            cells,
            page_rows,
            GridRowTotal::Exact(1_000_000),
            page_rows as u64,
            false,
        );
        grid.operation = GridOperationState::Streaming;
        grid.viewport_row = 0;
        // Resident window only.
        assert_eq!(grid.cells.len(), page_rows as usize);
        assert!(grid.is_resident(0));
        assert!(grid.is_resident(99));
        assert!(!grid.is_resident(100));
        assert!(grid.needs_fetch(100));
        assert!(!grid.needs_fetch(50));
        // Jump viewport far past resident page — still O(1) checks.
        grid.viewport_row = 999_900;
        assert!(!grid.is_resident(999_900));
        assert!(grid.needs_fetch(999_900));
        assert_eq!(grid.totals, GridRowTotal::Exact(1_000_000));
        assert!(grid.status_line().contains("total 1000000") || grid.status_line().contains("1000000"));
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
    fn temporal_step_day_preserves_time_suffix() {
        let mut session = CellEditSession {
            abs_row: 0,
            column: "ts".into(),
            original_text: "2024-01-15T12:30:00Z".into(),
            buffer: "2024-01-15T12:30:00Z".into(),
            locator: Vec::new(),
            kind: CellDistinction::Temporal,
        };
        assert!(session.step_day(1));
        assert_eq!(session.buffer, "2024-01-16T12:30:00Z");
        assert!(session.step_day(-2));
        assert_eq!(session.buffer, "2024-01-14T12:30:00Z");
        // Month boundary
        session.buffer = "2024-01-31".into();
        assert!(session.step_day(1));
        assert_eq!(session.buffer, "2024-02-01");
        // Invalid date text: no-op
        session.buffer = "not-a-date".into();
        assert!(!session.step_day(1));
        let mut text = CellEditSession {
            abs_row: 0,
            column: "t".into(),
            original_text: String::new(),
            buffer: "2024-01-01".into(),
            locator: Vec::new(),
            kind: CellDistinction::Text,
        };
        assert!(!text.step_day(1));
    }

    #[test]
    fn temporal_step_month_and_calendar_text() {
        let mut session = CellEditSession {
            abs_row: 0,
            column: "ts".into(),
            original_text: "2024-01-31T12:00:00Z".into(),
            buffer: "2024-01-31T12:00:00Z".into(),
            locator: Vec::new(),
            kind: CellDistinction::Temporal,
        };
        assert!(session.step_month(1));
        // Jan 31 → Feb 29 (2024 leap)
        assert_eq!(session.buffer, "2024-02-29T12:00:00Z");
        assert!(session.step_month(-1));
        assert_eq!(session.buffer, "2024-01-29T12:00:00Z");
        let cal = session.month_calendar_text().expect("calendar");
        assert!(cal.contains("January 2024"), "{cal}");
        assert!(cal.contains("Su Mo Tu We Th Fr Sa"), "{cal}");
        assert!(cal.contains("*29") || cal.contains(" 29"), "{cal}");
        // Non-leap Feb clamp
        session.buffer = "2023-01-31".into();
        assert!(session.step_month(1));
        assert_eq!(session.buffer, "2023-02-28");
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
