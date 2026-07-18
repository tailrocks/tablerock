//! Connections-screen profile list submodel.

use crate::effect::RequestToken;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRowProjection {
    pub id_hex: String,
    pub name: String,
    pub engine_label: String,
    pub group: Option<String>,
    pub favorite: bool,
    /// `host:port/database` (Redis: logical DB index as database).
    pub target_summary: String,
    pub environment: Option<String>,
    pub production_warning: bool,
    pub safety_label: String,
    pub plaintext_secret_warning: bool,
    pub live_state: LiveConnectionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

impl LiveConnectionState {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Disconnected => "disconnected",
            Self::Connecting => "connecting",
            Self::Connected => "connected",
            Self::Reconnecting => "reconnecting",
            Self::Failed => "failed",
        }
    }
}

impl ProfileRowProjection {
    #[must_use]
    pub fn list_line(&self) -> String {
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
        let secret = if self.plaintext_secret_warning {
            " *plaintext*"
        } else {
            ""
        };
        format!(
            "{}  {}  {}  {}  {}{env}{secret}",
            self.engine_label,
            self.name,
            self.target_summary,
            self.safety_label,
            self.live_state.label(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureProjection {
    /// Safe, redacted operator-facing label (no secrets, no SQL).
    Label(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileListState {
    Idle,
    Loading {
        request_token: RequestToken,
    },
    Loaded {
        request_token: RequestToken,
        rows: Vec<ProfileRowProjection>,
        /// Selected row in the current (search-filtered) view.
        selected: usize,
        /// Client-side search text (name/host/group).
        search: String,
    },
    Failed {
        request_token: RequestToken,
        reason: FailureProjection,
    },
}

impl Default for ProfileListState {
    fn default() -> Self {
        Self::Idle
    }
}

impl ProfileListState {
    #[must_use]
    pub fn status_line(&self) -> String {
        match self {
            Self::Idle => "Profiles: —".to_owned(),
            Self::Loading { .. } => "Profiles: loading…".to_owned(),
            Self::Loaded { rows, search, .. } => {
                let visible = Self::filter_rows(rows, search).len();
                if search.is_empty() {
                    format!("Profiles: {visible}")
                } else {
                    format!("Profiles: {visible}/{} (filter)", rows.len())
                }
            }
            Self::Failed {
                reason: FailureProjection::Label(label),
                ..
            } => format!("Profiles: error ({label})"),
        }
    }

    #[must_use]
    pub const fn active_token(&self) -> Option<RequestToken> {
        match self {
            Self::Idle => None,
            Self::Loading { request_token }
            | Self::Loaded { request_token, .. }
            | Self::Failed { request_token, .. } => Some(*request_token),
        }
    }

    #[must_use]
    pub fn visible_rows(&self) -> Vec<&ProfileRowProjection> {
        match self {
            Self::Loaded { rows, search, .. } => Self::filter_rows(rows, search),
            _ => Vec::new(),
        }
    }

    #[must_use]
    pub fn selected_row(&self) -> Option<&ProfileRowProjection> {
        match self {
            Self::Loaded {
                rows,
                selected,
                search,
                ..
            } => {
                let visible = Self::filter_rows(rows, search);
                visible.get(*selected).copied()
            }
            _ => None,
        }
    }

    pub fn select_next(&mut self) {
        if let Self::Loaded {
            rows,
            selected,
            search,
            ..
        } = self
        {
            let len = Self::filter_rows(rows, search).len();
            if len == 0 {
                *selected = 0;
            } else {
                *selected = (*selected + 1) % len;
            }
        }
    }

    pub fn select_previous(&mut self) {
        if let Self::Loaded {
            rows,
            selected,
            search,
            ..
        } = self
        {
            let len = Self::filter_rows(rows, search).len();
            if len == 0 {
                *selected = 0;
            } else if *selected == 0 {
                *selected = len - 1;
            } else {
                *selected -= 1;
            }
        }
    }

    pub fn push_search(&mut self, text: &str) {
        if let Self::Loaded {
            selected, search, ..
        } = self
        {
            search.push_str(text);
            *selected = 0;
        }
    }

    pub fn clear_search(&mut self) {
        if let Self::Loaded {
            selected, search, ..
        } = self
        {
            search.clear();
            *selected = 0;
        }
    }

    fn filter_rows<'a>(
        rows: &'a [ProfileRowProjection],
        search: &str,
    ) -> Vec<&'a ProfileRowProjection> {
        let needle = search.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return rows.iter().collect();
        }
        rows.iter()
            .filter(|row| {
                row.name.to_ascii_lowercase().contains(&needle)
                    || row.target_summary.to_ascii_lowercase().contains(&needle)
                    || row
                        .group
                        .as_deref()
                        .is_some_and(|group| group.to_ascii_lowercase().contains(&needle))
                    || row.engine_label.to_ascii_lowercase().contains(&needle)
            })
            .collect()
    }
}
