//! Workbench shell submodel (TableRock-local; TermRock widgets render it).

use super::catalog::CatalogModel;

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

/// One workbench tab (content is placeholder until plans 009/011).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchTab {
    pub id: u64,
    pub title: String,
    pub dirty: bool,
    pub running: bool,
    pub preview: bool,
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

    pub fn bump_context_revision(&mut self) -> u64 {
        self.context_revision = self.context_revision.saturating_add(1);
        self.context_revision
    }
}
