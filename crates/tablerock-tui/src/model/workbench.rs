//! Workbench shell submodel (TableRock-local; TermRock widgets render it).

use super::catalog::CatalogModel;
use super::grid::DataGridModel;
use super::inspector::InspectorModel;

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

/// One workbench tab (grid content for data tabs; SQL later).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchTab {
    pub id: u64,
    pub title: String,
    pub dirty: bool,
    pub running: bool,
    pub preview: bool,
    pub grid: DataGridModel,
    /// Optional SQL text for a statement tab.
    pub sql: Option<String>,
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
                sql: None,
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
            sql: None,
        });
        self.selected_tab = self.tabs.len() - 1;
    }

    /// Open a SQL statement tab (single-line input until plan 011).
    pub fn open_sql_tab(&mut self) {
        let id = self.tabs.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        self.tabs.push(WorkbenchTab {
            id,
            title: "SQL".into(),
            dirty: false,
            running: false,
            preview: false,
            grid: DataGridModel::default(),
            sql: Some(String::new()),
        });
        self.selected_tab = self.tabs.len() - 1;
    }

    pub fn active_grid_mut(&mut self) -> Option<&mut DataGridModel> {
        self.tabs.get_mut(self.selected_tab).map(|t| &mut t.grid)
    }

    pub fn active_grid(&self) -> Option<&DataGridModel> {
        self.tabs.get(self.selected_tab).map(|t| &t.grid)
    }

    /// Open inspector for the cursor cell of the active grid.
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
        self.inspector = InspectorModel::from_cell(title, &cell, false);
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

    /// Mark every live tab disconnected without dropping content.
    pub fn mark_disconnected(&mut self) {
        self.context.health_label = "disconnected".into();
        for tab in &mut self.tabs {
            if tab.running {
                tab.running = false;
            }
        }
        self.status.operation = "disconnected".into();
    }

    fn recompute_pending(&mut self) {
        self.status.pending_changes = self.tabs.iter().filter(|t| t.dirty).count() as u32;
    }
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
}
