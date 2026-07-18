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

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: i64, preview: &str) -> HistoryRowProjection {
        HistoryRowProjection {
            history_id: id,
            engine_label: "PostgreSQL".into(),
            database: "db".into(),
            schema: Some("public".into()),
            statement_preview: preview.into(),
            outcome: "ok".into(),
            created_at: "2026-07-18".into(),
        }
    }

    fn open(entries: Vec<HistoryRowProjection>) -> HistoryPanel {
        HistoryPanel::Open {
            request_token: 1,
            entries,
            selected: 0,
            search: String::new(),
        }
    }

    #[test]
    fn closed_loading_failed_status_and_openness() {
        let closed = HistoryPanel::Closed;
        assert!(!closed.is_open());
        assert_eq!(closed.status_line(), "");
        assert!(closed.selected_entry().is_none());

        let loading = HistoryPanel::Loading { request_token: 2 };
        assert!(loading.is_open());
        assert_eq!(loading.status_line(), "History: loading…");
        assert!(loading.selected_entry().is_none());

        let failed = HistoryPanel::Failed {
            request_token: 3,
            reason: "disk".into(),
        };
        assert!(failed.is_open());
        assert_eq!(failed.status_line(), "History: error (disk)");
        assert!(failed.selected_entry().is_none());
    }

    #[test]
    fn selection_wraps_and_status_tracks_position() {
        let mut m = open(vec![row(1, "select 1"), row(2, "select 2"), row(3, "select 3")]);
        assert_eq!(m.selected_entry().unwrap().history_id, 1);
        assert_eq!(m.status_line(), "History: 1/3");

        m.select_next();
        assert_eq!(m.selected_entry().unwrap().history_id, 2);
        assert_eq!(m.status_line(), "History: 2/3");

        m.select_next();
        m.select_next(); // 2 -> 3 -> wrap to 1
        assert_eq!(m.selected_entry().unwrap().history_id, 1);

        m.select_prev(); // wrap back to 3
        assert_eq!(m.selected_entry().unwrap().history_id, 3);
    }

    #[test]
    fn empty_open_navigation_is_safe_without_selection() {
        let mut m = open(Vec::new());
        m.select_next();
        m.select_prev();
        assert!(m.selected_entry().is_none());
    }
}
