//! Presentation projections for query history.

/// One history row for the workbench panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRowProjection {
    pub history_id: i64,
    pub engine_label: String,
    pub database: String,
    pub schema: Option<String>,
    pub statement_preview: String,
    pub outcome: String,
    pub created_at: String,
}

/// History panel state (local presentation).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HistoryPanel {
    #[default]
    Closed,
    Loading {
        request_token: u64,
    },
    Open {
        request_token: u64,
        entries: Vec<HistoryRowProjection>,
        selected: usize,
        search: String,
    },
    Failed {
        request_token: u64,
        reason: String,
    },
}

impl HistoryPanel {
    #[must_use]
    pub fn is_open(&self) -> bool {
        !matches!(self, Self::Closed)
    }

    pub fn select_next(&mut self) {
        if let Self::Open {
            entries, selected, ..
        } = self
            && !entries.is_empty()
        {
            *selected = (*selected + 1) % entries.len();
        }
    }

    pub fn select_prev(&mut self) {
        if let Self::Open {
            entries, selected, ..
        } = self
            && !entries.is_empty()
        {
            if *selected == 0 {
                *selected = entries.len() - 1;
            } else {
                *selected -= 1;
            }
        }
    }

    #[must_use]
    pub fn selected_entry(&self) -> Option<&HistoryRowProjection> {
        match self {
            Self::Open {
                entries, selected, ..
            } => entries.get(*selected),
            _ => None,
        }
    }

    #[must_use]
    pub fn status_line(&self) -> String {
        match self {
            Self::Closed => String::new(),
            Self::Loading { .. } => "History: loading…".into(),
            Self::Open {
                entries, selected, ..
            } => {
                format!("History: {}/{}", selected.saturating_add(1), entries.len())
            }
            Self::Failed { reason, .. } => format!("History: error ({reason})"),
        }
    }
}
