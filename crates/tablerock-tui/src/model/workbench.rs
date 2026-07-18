//! Workbench shell submodel (TableRock-local; TermRock widgets render it).

use super::catalog::CatalogModel;
use super::completion::CompletionSession;
use super::grid::DataGridModel;
use super::history::HistoryPanel;
use super::inspector::InspectorModel;
use super::mutation_plan_build::MutationReviewView;
use super::query_editor::QueryEditorModel;
use super::result_sections::ResultSectionsModel;
use super::saved_query::{BoundSqlFile, SavedQueryPanel};
use tablerock_core::SqlDialect;

/// Context-bar projection for the active session.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContextBarModel {
    pub connection_name: String,
    pub engine_label: String,
    pub database: String,
    pub schema: Option<String>,
    pub environment: Option<String>,
    pub production_warning: bool,
    pub safety_label: String,
    pub health_label: String,
}

impl ContextBarModel {
    #[must_use]
    pub fn line(&self) -> String {
        let env = self
            .environment
            .as_deref()
            .map(|value| {
                if self.production_warning {
                    format!(" [{value}!]")
                } else {
                    format!(" [{value}]")
                }
            })
            .unwrap_or_default();
        let schema = self
            .schema
            .as_deref()
            .map(|value| format!(" · schema {value}"))
            .unwrap_or_default();
        format!(
            "{} · {} · db {}{}{} · {} · {}",
            self.connection_name,
            self.engine_label,
            self.database,
            schema,
            env,
            self.safety_label,
            self.health_label
        )
    }
}

/// One workbench tab (grid content and/or SQL editor).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchTab {
    pub id: u64,
    pub title: String,
    pub dirty: bool,
    pub running: bool,
    pub preview: bool,
    pub grid: DataGridModel,
    /// Multiline SQL editor when this is a statement tab.
    pub editor: Option<QueryEditorModel>,
    /// Bound `.sql` path for this tab, if opened/saved as a file.
    pub bound_file: Option<BoundSqlFile>,
}

/// Status-bar facts for the active tab/session.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkbenchStatus {
    pub operation: String,
    pub rows: u64,
    pub bytes: u64,
    pub truncated: bool,
    pub pending_changes: u32,
}

impl WorkbenchStatus {
    #[must_use]
    pub fn summary(&self) -> String {
        let trunc = if self.truncated { " trunc" } else { "" };
        format!(
            "{} · {} rows · {} B{} · pending {}",
            self.operation, self.rows, self.bytes, trunc, self.pending_changes
        )
    }
}

/// Root workbench shell state after Connect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchModel {
    pub context: ContextBarModel,
    pub tabs: Vec<WorkbenchTab>,
    pub selected_tab: usize,
    pub status: WorkbenchStatus,
    pub context_revision: u64,
    pub catalog: CatalogModel,
    /// Engine kind string for catalog root request mapping.
    pub engine_kind: String,
    pub inspector: InspectorModel,
    /// Open completion popup for the active SQL editor, if any.
    pub completion: Option<CompletionSession>,
    pub history: HistoryPanel,
    /// History retention policy projection: "full" | "metadata" | "private".
    pub history_retention: String,
    pub saved_queries: SavedQueryPanel,
    /// Profile id for intent-only session restore (hex), when non-temporary.
    pub profile_id_hex: Option<String>,
    /// Named filter presets for this profile (loaded/saved via persistence).
    pub filter_library: crate::model::saved_filter::SavedFilterLibrary,
    /// Open mutation review dialog (typed plan preview; never executed text).
    pub mutation_review: Option<MutationReviewView>,
    /// Handle from MutationReviewReady; required for ApplyMutations (consume-once).
    pub pending_review_token_hex: Option<String>,
    /// Wall-clock expiry (ms) for the pending review token (display only).
    pub pending_review_expires_at_ms: Option<u64>,
    /// Multi-statement run section panel (one row per statement).
    pub result_sections: ResultSectionsModel,
    /// Active Redis key for collection staging: (logical_db, key, kind_label).
    pub redis_stage_target: Option<(String, String, String)>,
    /// Staged Redis collection mutation specs (review/apply handle path).
    pub redis_staged: Vec<crate::effect::MutationChangeSpec>,
    /// Next collection skip for RMore (hash/set/zset pagination).
    pub redis_collection_skip: Option<u64>,
}

impl Default for WorkbenchModel {
    fn default() -> Self {
        Self {
            context: ContextBarModel {
                connection_name: "session".into(),
                engine_label: "PostgreSQL".into(),
                database: "postgres".into(),
                schema: Some("public".into()),
                environment: None,
                production_warning: false,
                safety_label: "Confirm writes".into(),
                health_label: "connected".into(),
            },
            tabs: vec![WorkbenchTab {
                id: 1,
                title: "Welcome".into(),
                dirty: false,
                running: false,
                preview: true,
                grid: DataGridModel::default(),
                editor: None,
                bound_file: None,
            }],
            selected_tab: 0,
            status: WorkbenchStatus {
                operation: "idle".into(),
                rows: 0,
                bytes: 0,
                truncated: false,
                pending_changes: 0,
            },
            context_revision: 1,
            catalog: CatalogModel::Idle,
            engine_kind: "PostgreSQL".into(),
            inspector: InspectorModel::default(),
            completion: None,
            history: HistoryPanel::Closed,
            history_retention: "full".into(),
            saved_queries: SavedQueryPanel::Closed,
            profile_id_hex: None,
            filter_library: crate::model::saved_filter::SavedFilterLibrary::default(),
            mutation_review: None,
            pending_review_token_hex: None,
            pending_review_expires_at_ms: None,
            result_sections: ResultSectionsModel::default(),
            redis_stage_target: None,
            redis_staged: Vec::new(),
            redis_collection_skip: None,
        }
    }
}

impl WorkbenchModel {
    #[must_use]
    pub fn from_session(
        connection_name: impl Into<String>,
        engine_label: impl Into<String>,
        temporary: bool,
        identity: impl Into<String>,
    ) -> Self {
        let engine_label = engine_label.into();
        let mut model = Self::default();
        model.context.connection_name = connection_name.into();
        model.context.engine_label = engine_label.clone();
        model.engine_kind = engine_label;
        model.context.health_label = "connected".into();
        model.status.operation = if temporary {
            format!("temporary · {}", identity.into())
        } else {
            format!("connected · {}", identity.into())
        };
        model.catalog = CatalogModel::Idle;
        model
    }

    #[must_use]
    pub fn catalog_status_line(&self) -> String {
        self.catalog.status_line()
    }

    #[must_use]
    pub fn active_tab(&self) -> Option<&WorkbenchTab> {
        self.tabs.get(self.selected_tab)
    }

    pub fn select_next_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.selected_tab = (self.selected_tab + 1) % self.tabs.len();
    }

    pub fn select_previous_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        if self.selected_tab == 0 {
            self.selected_tab = self.tabs.len() - 1;
        } else {
            self.selected_tab -= 1;
        }
    }

    /// Wrap-around previous/next for tests and callers.
    #[must_use]
    pub fn selected_tab_index(&self) -> usize {
        self.selected_tab
    }

    pub fn bump_context_revision(&mut self) -> u64 {
        self.context_revision = self.context_revision.saturating_add(1);
        self.context_revision
    }

    /// Open or focus a preview tab for an object title.
    pub fn open_preview_tab(&mut self, title: impl Into<String>) {
        let title = title.into();
        if let Some(index) = self
            .tabs
            .iter()
            .position(|tab| tab.title == title && tab.preview)
        {
            self.selected_tab = index;
            return;
        }
        let id = self.tabs.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        self.tabs.push(WorkbenchTab {
            id,
            title,
            dirty: false,
            running: false,
            preview: true,
            grid: DataGridModel::default(),
            editor: None,
            bound_file: None,
        });
        self.selected_tab = self.tabs.len() - 1;
    }

    /// Open a multiline SQL / Redis command editor tab (TermRock TextArea).
    pub fn open_sql_tab(&mut self) {
        let (dialect, title) = match self.engine_kind.as_str() {
            "ClickHouse" => (SqlDialect::ClickHouse, "SQL"),
            "Redis" => (SqlDialect::PostgreSql, "Redis"), // text editor; no SQL parse path
            _ => (SqlDialect::PostgreSql, "SQL"),
        };
        let id = self.tabs.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        self.tabs.push(WorkbenchTab {
            id,
            title: title.into(),
            dirty: false,
            running: false,
            preview: false,
            grid: DataGridModel::default(),
            editor: Some(QueryEditorModel::new(dialect)),
            bound_file: None,
        });
        self.selected_tab = self.tabs.len() - 1;
    }

    #[must_use]
    pub fn active_editor(&self) -> Option<&QueryEditorModel> {
        self.tabs
            .get(self.selected_tab)
            .and_then(|t| t.editor.as_ref())
    }

    pub fn active_editor_mut(&mut self) -> Option<&mut QueryEditorModel> {
        self.tabs
            .get_mut(self.selected_tab)
            .and_then(|t| t.editor.as_mut())
    }

    /// Open or refresh completion from the active editor + catalog.
    ///
    /// Redis uses the curated command table (not SQL keywords/catalog).
    pub fn open_completion(&mut self) {
        let Some(editor) = self.active_editor() else {
            self.completion = None;
            return;
        };
        // Clone facts for pure build without dual borrow.
        let editor = editor.clone();
        let context_revision = self.context_revision;
        if self.engine_kind.eq_ignore_ascii_case("Redis") {
            self.completion = Some(super::completion::build_redis_session(
                &editor,
                context_revision,
            ));
            return;
        }
        let catalog = self.catalog.clone();
        self.completion = Some(super::completion::build_session(
            &editor,
            &catalog,
            context_revision,
        ));
    }

    pub fn dismiss_completion(&mut self) {
        self.completion = None;
    }

    /// Build intent-only JSON for the active workbench (tabs + context text).
    #[must_use]
    pub fn intent_json(&self) -> String {
        let mut tabs = String::from("[");
        for (i, tab) in self.tabs.iter().enumerate() {
            if i > 0 {
                tabs.push(',');
            }
            let title = json_escape(&tab.title);
            let sql = tab
                .editor
                .as_ref()
                .map(|e| format!("\"{}\"", json_escape(e.text())))
                .unwrap_or_else(|| "null".into());
            tabs.push_str(&format!(r#"{{"title":"{title}","sql":{sql}}}"#));
        }
        tabs.push(']');
        let schema = self
            .context
            .schema
            .as_ref()
            .map(|s| format!("\"{}\"", json_escape(s)))
            .unwrap_or_else(|| "null".into());
        format!(
            r#"{{"database":"{}","schema":{schema},"selected_tab":{},"tabs":{tabs}}}"#,
            json_escape(&self.context.database),
            self.selected_tab,
        )
    }

    /// Apply intent-only JSON: database/schema + SQL tab texts (never results).
    pub fn apply_intent_json(&mut self, json: &str) -> bool {
        if json.contains("\"cells\"") || json.contains("\"result_pages\"") {
            return false;
        }
        // Minimal hand parser for our controlled shape (avoid serde dep in tui).
        if let Some(db) = extract_json_string(json, "database") {
            self.context.database = db;
        }
        if let Some(schema) = extract_json_string(json, "schema") {
            self.context.schema = Some(schema);
        } else if json.contains("\"schema\":null") {
            self.context.schema = None;
        }
        // Restore SQL tabs from "sql":"..." pairs with preceding title.
        let mut restored = Vec::new();
        let mut rest = json;
        while let Some(title_idx) = rest.find("\"title\"") {
            rest = &rest[title_idx..];
            let Some(title) = extract_json_string(rest, "title") else {
                break;
            };
            let sql = extract_json_string(rest, "sql");
            restored.push((title, sql));
            rest = rest.get(8..).unwrap_or("");
        }
        if restored.is_empty() {
            return true;
        }
        // Replace non-preview tabs with restored SQL tabs; keep one welcome if empty.
        self.tabs.retain(|t| t.preview && t.title == "Welcome");
        if self.tabs.is_empty() {
            self.tabs.push(WorkbenchTab {
                id: 1,
                title: "Welcome".into(),
                dirty: false,
                running: false,
                preview: true,
                grid: DataGridModel::default(),
                editor: None,
                bound_file: None,
            });
        }
        let dialect = match self.engine_kind.as_str() {
            "ClickHouse" => SqlDialect::ClickHouse,
            _ => SqlDialect::PostgreSql,
        };
        for (title, sql) in restored {
            if sql.is_none() && title == "Welcome" {
                continue;
            }
            let id = self.tabs.iter().map(|t| t.id).max().unwrap_or(0) + 1;
            let mut editor = QueryEditorModel::new(dialect);
            if let Some(text) = sql {
                editor.set_text(text);
            }
            self.tabs.push(WorkbenchTab {
                id,
                title,
                dirty: false,
                running: false,
                preview: false,
                grid: DataGridModel::default(),
                editor: Some(editor),
                bound_file: None,
            });
        }
        if let Some(sel) = extract_json_number(json, "selected_tab") {
            let sel = sel as usize;
            if sel < self.tabs.len() {
                self.selected_tab = sel;
            }
        }
        true
    }

    pub fn commit_completion(&mut self, candidate_id: Option<&str>) -> bool {
        let Some(session) = self.completion.clone() else {
            return false;
        };
        let id = candidate_id
            .map(str::to_owned)
            .or_else(|| session.selected_id.clone());
        let Some(id) = id else {
            return false;
        };
        // Redis sessions pin catalog_revision = 0 (no schema catalog axis).
        let catalog_rev = if self.engine_kind.eq_ignore_ascii_case("Redis") {
            0
        } else {
            super::completion::catalog_revision(&self.catalog)
        };
        let context_revision = self.context_revision;
        let Some(editor) = self.active_editor_mut() else {
            self.completion = None;
            return false;
        };
        match super::completion::commit_candidate(
            editor,
            &session,
            &id,
            context_revision,
            catalog_rev,
        ) {
            Ok(()) => {
                if let Some(tab) = self.tabs.get_mut(self.selected_tab) {
                    tab.dirty = true;
                }
                self.completion = None;
                true
            }
            Err(_) => {
                self.completion = None;
                false
            }
        }
    }

    pub fn active_grid_mut(&mut self) -> Option<&mut DataGridModel> {
        self.tabs.get_mut(self.selected_tab).map(|t| &mut t.grid)
    }

    pub fn active_grid(&self) -> Option<&DataGridModel> {
        self.tabs.get(self.selected_tab).map(|t| &t.grid)
    }

    /// Open inspector for the cursor cell of the active grid.
    ///
    /// Toggles closed when the inspector is already open on the same cursor title.
    /// When drafts exist, appends staged/original/delete facts so originals stay
    /// reachable without color alone (product editing requirement).
    pub fn inspect_cursor(&mut self) {
        let Some(grid) = self.active_grid() else {
            self.inspector = InspectorModel::default();
            return;
        };
        let cell = grid.cell_at(grid.cursor_row, grid.cursor_col);
        let col_name = grid
            .columns
            .get(grid.cursor_col)
            .cloned()
            .unwrap_or_else(|| format!("col{}", grid.cursor_col));
        let title = format!("r{} · {col_name}", grid.cursor_row);
        if self.inspector.open && self.inspector.title == title {
            self.inspector = InspectorModel::default();
            return;
        }
        let mut insp = InspectorModel::from_cell(title, &cell, false);
        let row = grid.cursor_row;
        let mut draft_lines: Vec<String> = Vec::new();
        let row_marker = grid.drafts.row_marker(row);
        if matches!(
            row_marker,
            crate::model::mutation_draft::DraftMarker::Deleted
        ) {
            draft_lines.push(format!(
                "draft: {} (row staged for delete)",
                row_marker.label()
            ));
        }
        if let Some(staged) = grid.drafts.staged_for_cell(row, &col_name) {
            draft_lines.push(format!("staged: {staged}"));
            if let Some(orig) = grid.drafts.original_for_cell(row, &col_name) {
                draft_lines.push(format!("original: {orig}"));
                draft_lines.push(compare_original_staged(orig, staged));
            }
        } else if matches!(
            row_marker,
            crate::model::mutation_draft::DraftMarker::Modified
        ) {
            draft_lines.push("draft: modified row (other cells staged)".into());
        }
        if let Some(edit) = grid.cell_edit.as_ref() {
            if edit.abs_row == row && edit.column == col_name {
                draft_lines.push(format!("editing: {}", edit.buffer));
            }
        }
        if !draft_lines.is_empty() {
            insp.text = format!("{}\n{}", insp.text, draft_lines.join("\n"));
        }
        self.inspector = insp;
    }

    /// Force-close the inspector panel.
    pub fn close_inspector(&mut self) {
        self.inspector = InspectorModel::default();
    }

    /// Promote the active preview tab to durable (edit/pin/filter/sort).
    pub fn promote_active_tab(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.selected_tab) {
            tab.preview = false;
        }
    }

    /// Mark active tab dirty (staged changes or unsaved text).
    pub fn mark_active_dirty(&mut self, dirty: bool) {
        if let Some(tab) = self.tabs.get_mut(self.selected_tab) {
            tab.dirty = dirty;
            if dirty {
                tab.preview = false;
            }
        }
        self.recompute_pending();
    }

    pub fn mark_active_running(&mut self, running: bool) {
        if let Some(tab) = self.tabs.get_mut(self.selected_tab) {
            tab.running = running;
        }
        self.status.operation = if running {
            "running".into()
        } else {
            "idle".into()
        };
    }

    /// Close active tab. Returns `NeedsConfirm` when dirty.
    pub fn close_active_tab(&mut self) -> CloseTabOutcome {
        if self.tabs.is_empty() {
            return CloseTabOutcome::Empty;
        }
        if self.tabs[self.selected_tab].dirty {
            return CloseTabOutcome::NeedsConfirm {
                title: self.tabs[self.selected_tab].title.clone(),
                index: self.selected_tab,
            };
        }
        self.force_close_tab(self.selected_tab);
        CloseTabOutcome::Closed
    }

    pub fn force_close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.selected_tab = 0;
        } else if self.selected_tab >= self.tabs.len() {
            self.selected_tab = self.tabs.len() - 1;
        }
        self.recompute_pending();
    }

    /// Mark every live tab/operation disconnected without dropping content.
    ///
    /// Completed/failed grids stay inspectable with their prior terminal state;
    /// only live (queued/running/streaming/cancel-*) operations flip to
    /// [`GridOperationState::Disconnected`].
    pub fn mark_disconnected(&mut self) {
        self.context.health_label = "disconnected".into();
        for tab in &mut self.tabs {
            if tab.running {
                tab.running = false;
            }
            tab.grid.mark_disconnected();
        }
        self.status.operation = "disconnected".into();
    }

    fn recompute_pending(&mut self) {
        self.status.pending_changes = self.tabs.iter().filter(|t| t.dirty).count() as u32;
    }
}

fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let idx = json.find(&needle)?;
    let after = &json[idx + needle.len()..];
    let after = after.trim_start();
    let after = after.strip_prefix(':')?.trim_start();
    if after.starts_with("null") {
        return None;
    }
    let after = after.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = after.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(code) = u32::from_str_radix(&hex, 16) {
                        if let Some(c) = char::from_u32(code) {
                            out.push(c);
                        }
                    }
                }
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
    None
}

fn extract_json_number(json: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\"");
    let idx = json.find(&needle)?;
    let after = &json[idx + needle.len()..];
    let after = after.trim_start().strip_prefix(':')?.trim_start();
    let digits: String = after
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseTabOutcome {
    Empty,
    Closed,
    NeedsConfirm { title: String, index: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_promotes_and_close_dirty_asks() {
        let mut wb = WorkbenchModel::default();
        wb.open_preview_tab("users");
        assert!(wb.active_tab().unwrap().preview);
        wb.mark_active_dirty(true);
        assert!(!wb.active_tab().unwrap().preview);
        assert!(wb.active_tab().unwrap().dirty);
        assert_eq!(wb.status.pending_changes, 1);
        assert!(matches!(
            wb.close_active_tab(),
            CloseTabOutcome::NeedsConfirm { .. }
        ));
        wb.force_close_tab(wb.selected_tab);
        assert!(wb.tabs.iter().all(|t| t.title != "users"));
    }

    #[test]
    fn next_and_previous_tab_wrap() {
        let mut wb = WorkbenchModel::default();
        let base = wb.tabs.len();
        wb.open_preview_tab("a");
        wb.open_preview_tab("b");
        wb.open_preview_tab("c");
        assert_eq!(wb.tabs.len(), base + 3);
        let last = wb.tabs.len() - 1;
        assert_eq!(wb.selected_tab_index(), last);
        wb.select_next_tab();
        assert_eq!(wb.selected_tab_index(), 0);
        wb.select_previous_tab();
        assert_eq!(wb.selected_tab_index(), last);
        wb.select_previous_tab();
        assert_eq!(wb.selected_tab_index(), last - 1);
    }

    #[test]
    fn redis_open_completion_uses_command_table() {
        let mut wb = WorkbenchModel::default();
        wb.engine_kind = "Redis".into();
        wb.open_sql_tab();
        {
            let ed = wb.active_editor_mut().unwrap();
            ed.set_text("SET");
            ed.set_cursor(3);
        }
        wb.open_completion();
        let session = wb.completion.as_ref().expect("session");
        assert!(
            session
                .candidates
                .iter()
                .any(|c| c.label == "SET" && c.id.starts_with("redis:")),
            "{:?}",
            session.candidates
        );
        assert!(!session.candidates.iter().any(|c| c.label == "SELECT"));
        assert!(wb.commit_completion(Some("redis:SET")));
        assert!(wb.active_editor().unwrap().text().starts_with("SET"));
    }

    #[test]
    fn intent_json_round_trip_restores_sql_tabs_not_results() {
        let mut wb = WorkbenchModel::default();
        wb.context.database = "app".into();
        wb.context.schema = Some("public".into());
        wb.open_sql_tab();
        wb.active_editor_mut()
            .unwrap()
            .set_text("SELECT 99 FROM t");
        let json = wb.intent_json();
        assert!(json.contains("SELECT 99"));
        assert!(!json.contains("cells"));

        let mut other = WorkbenchModel::default();
        other.engine_kind = "PostgreSQL".into();
        assert!(other.apply_intent_json(&json));
        assert_eq!(other.context.database, "app");
        assert_eq!(other.context.schema.as_deref(), Some("public"));
        let sql_tabs: Vec<_> = other
            .tabs
            .iter()
            .filter(|t| t.editor.is_some())
            .collect();
        assert!(!sql_tabs.is_empty());
        assert!(
            sql_tabs
                .iter()
                .any(|t| t.editor.as_ref().unwrap().text().contains("SELECT 99"))
        );
        // Reject result-shaped payloads.
        assert!(!other.apply_intent_json(r#"{"database":"x","cells":[]}"#));
    }

    #[test]
    fn disconnect_keeps_tabs_inspectable() {
        let mut wb = WorkbenchModel::default();
        wb.open_preview_tab("orders");
        wb.mark_active_running(true);
        wb.mark_disconnected();
        assert_eq!(wb.context.health_label, "disconnected");
        assert_eq!(wb.tabs.len(), 2);
        assert!(!wb.tabs.iter().any(|t| t.running));
        assert_eq!(wb.status.operation, "disconnected");
    }

    #[test]
    fn disconnect_marks_live_grid_disconnected_and_keeps_cells() {
        use crate::model::grid::{
            CellDistinction, GridOperationState, GridRowTotal, ProjectedCell,
        };
        let mut wb = WorkbenchModel::default();
        wb.open_preview_tab("stream");
        if let Some(grid) = wb.active_grid_mut() {
            grid.operation = GridOperationState::Streaming;
            grid.base_schema = Some("public".into());
            grid.base_table = Some("stream".into());
            grid.columns = vec!["id".into()];
            grid.cells = vec![ProjectedCell {
                text: "7".into(),
                distinction: CellDistinction::Number,
                byte_len: 1,
                original_byte_len: None,
            }];
            grid.row_count = 1;
            grid.rows_loaded = 1;
            grid.totals = GridRowTotal::Unknown;
        }
        wb.mark_active_running(true);
        wb.mark_disconnected();
        let grid = wb.active_grid().expect("grid");
        assert_eq!(grid.operation, GridOperationState::Disconnected);
        assert_eq!(grid.cells[0].text, "7");
        assert_eq!(grid.row_count, 1);
        assert!(grid.status_line().contains("disconnected"));
    }

    #[test]
    fn disconnect_does_not_rewrite_completed_grid() {
        use crate::model::grid::GridOperationState;
        let mut wb = WorkbenchModel::default();
        wb.open_preview_tab("done");
        if let Some(grid) = wb.active_grid_mut() {
            grid.operation = GridOperationState::Completed;
            grid.row_count = 3;
        }
        wb.mark_disconnected();
        assert_eq!(
            wb.active_grid().unwrap().operation,
            GridOperationState::Completed
        );
    }

    #[test]
    fn inspect_cursor_toggles_closed_on_same_cell() {
        let mut wb = WorkbenchModel::default();
        wb.open_preview_tab("t");
        if let Some(grid) = wb.active_grid_mut() {
            grid.columns = vec!["id".into()];
            grid.row_count = 1;
            grid.cells = vec![crate::model::grid::ProjectedCell {
                text: "1".into(),
                distinction: crate::model::grid::CellDistinction::Number,
                byte_len: 1,
                original_byte_len: None,
            }];
            grid.cursor_row = 0;
            grid.cursor_col = 0;
        }
        wb.inspect_cursor();
        assert!(wb.inspector.open);
        assert!(wb.inspector.title.contains("id"));
        wb.inspect_cursor();
        assert!(!wb.inspector.open);
        wb.inspect_cursor();
        assert!(wb.inspector.open);
        wb.close_inspector();
        assert!(!wb.inspector.open);
    }

    #[test]
    fn inspect_cursor_shows_staged_and_original() {
        use crate::model::mutation_draft::{DraftLocatorField, StagedCellEdit};
        use tablerock_core::ProfileSafetyMode;

        let mut wb = WorkbenchModel::default();
        wb.open_preview_tab("t");
        if let Some(grid) = wb.active_grid_mut() {
            grid.columns = vec!["id".into(), "name".into()];
            grid.row_count = 1;
            grid.cells = vec![
                crate::model::grid::ProjectedCell {
                    text: "1".into(),
                    distinction: crate::model::grid::CellDistinction::Number,
                    byte_len: 1,
                    original_byte_len: None,
                },
                crate::model::grid::ProjectedCell {
                    text: "alice".into(),
                    distinction: crate::model::grid::CellDistinction::Text,
                    byte_len: 5,
                    original_byte_len: None,
                },
            ];
            grid.base_schema = Some("public".into());
            grid.base_table = Some("users".into());
            grid.identity_columns = vec!["id".into()];
            grid.recompute_editability(ProfileSafetyMode::ConfirmWrites, false);
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
            grid.cursor_row = 0;
            grid.cursor_col = 1;
        }
        wb.inspect_cursor();
        assert!(wb.inspector.open);
        assert!(
            wb.inspector.text.contains("staged: bob"),
            "{}",
            wb.inspector.text
        );
        assert!(
            wb.inspector.text.contains("original: alice"),
            "{}",
            wb.inspector.text
        );
        assert!(
            wb.inspector.text.contains("compare:"),
            "{}",
            wb.inspector.text
        );
        assert!(
            wb.inspector.text.contains("alice") && wb.inspector.text.contains("bob"),
            "{}",
            wb.inspector.text
        );
    }

    #[test]
    fn compare_original_staged_layout() {
        let block = compare_original_staged("alice", "bob");
        assert!(block.contains("compare:"), "{block}");
        assert!(block.contains("original"), "{block}");
        assert!(block.contains("staged"), "{block}");
        assert!(block.contains("alice"), "{block}");
        assert!(block.contains("bob"), "{block}");
        let multi = compare_original_staged("a\nb", "a\nc");
        assert!(multi.lines().count() >= 3, "{multi}");
    }
}

/// Side-by-side original | staged block for inspector (glyph+text, no color).
fn compare_original_staged(original: &str, staged: &str) -> String {
    let o_lines: Vec<&str> = if original.is_empty() {
        vec!["∅"]
    } else {
        original.lines().collect()
    };
    let s_lines: Vec<&str> = if staged.is_empty() {
        vec!["∅"]
    } else {
        staged.lines().collect()
    };
    let rows = o_lines.len().max(s_lines.len()).max(1);
    let o_w = o_lines.iter().map(|l| l.chars().count()).max().unwrap_or(0).max("original".len());
    let mut out = Vec::with_capacity(rows + 2);
    out.push("compare:".into());
    out.push(format!(
        "  {:o_w$} | staged",
        "original",
        o_w = o_w
    ));
    for i in 0..rows {
        let o = o_lines.get(i).copied().unwrap_or("");
        let s = s_lines.get(i).copied().unwrap_or("");
        out.push(format!("  {o:o_w$} | {s}", o_w = o_w));
    }
    out.join("\n")
}
