//! Presentation projections for named saved queries and bound `.sql` files.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedQueryRow {
    pub query_id: i64,
    pub name: String,
    pub engine_label: String,
    pub statement_preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SavedQueryPanel {
    #[default]
    Closed,
    Loading {
        request_token: u64,
    },
    Open {
        request_token: u64,
        entries: Vec<SavedQueryRow>,
        selected: usize,
    },
    Failed {
        request_token: u64,
        reason: String,
    },
}

impl SavedQueryPanel {
    #[must_use]
    pub fn is_open(&self) -> bool {
        !matches!(self, Self::Closed)
    }

    #[must_use]
    pub fn selected(&self) -> Option<&SavedQueryRow> {
        match self {
            Self::Open {
                entries, selected, ..
            } => entries.get(*selected),
            _ => None,
        }
    }
}

/// Bound file path for the active SQL tab (external-change detection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundSqlFile {
    pub path: String,
    pub mtime_secs: Option<u64>,
    pub len: u64,
}
