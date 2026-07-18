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
    /// Match a URL draft to a saved profile by engine + `host:port/database`.
    #[must_use]
    pub fn matches_url_target(
        &self,
        engine_label: &str,
        host: &str,
        port: u16,
        database: &str,
    ) -> bool {
        if !self.engine_label.eq_ignore_ascii_case(engine_label) {
            return false;
        }
        let want = format!("{host}:{port}/{database}");
        self.target_summary == want || self.target_summary.eq_ignore_ascii_case(&want)
    }

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

/// Tree node identity for connection list (group branch or profile leaf).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionNodeId {
    Group(String),
    Profile(String),
}

impl ConnectionNodeId {
    #[must_use]
    pub fn as_key(&self) -> String {
        match self {
            Self::Group(name) => format!("g:{name}"),
            Self::Profile(id) => format!("p:{id}"),
        }
    }

    #[must_use]
    pub fn profile_id(&self) -> Option<&str> {
        match self {
            Self::Profile(id) => Some(id.as_str()),
            Self::Group(_) => None,
        }
    }
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
        /// Selected profile id (leaf), if any.
        selected_id: Option<String>,
        /// Client-side search text (name/host/group).
        search: String,
        /// Collapsed group names.
        collapsed: Vec<String>,
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
                selected_id: Some(id),
                search,
                ..
            } => Self::filter_rows(rows, search)
                .into_iter()
                .find(|row| row.id_hex == *id),
            _ => None,
        }
    }

    pub fn select_next(&mut self) {
        self.move_selection(1);
    }

    pub fn select_previous(&mut self) {
        self.move_selection(-1);
    }

    fn move_selection(&mut self, delta: isize) {
        let Self::Loaded {
            rows,
            selected_id,
            search,
            collapsed,
            ..
        } = self
        else {
            return;
        };
        let leaf_ids = Self::leaf_profile_ids(rows, search, collapsed);
        if leaf_ids.is_empty() {
            *selected_id = None;
            return;
        }
        let current = selected_id
            .as_ref()
            .and_then(|id| leaf_ids.iter().position(|candidate| candidate == id))
            .unwrap_or(0);
        let next = if delta < 0 {
            if current == 0 {
                leaf_ids.len() - 1
            } else {
                current - 1
            }
        } else {
            (current + 1) % leaf_ids.len()
        };
        *selected_id = Some(leaf_ids[next].clone());
    }

    pub fn push_search(&mut self, text: &str) {
        if let Self::Loaded {
            selected_id,
            search,
            rows,
            collapsed,
            ..
        } = self
        {
            search.push_str(text);
            let leaves = Self::leaf_profile_ids(rows, search, collapsed);
            *selected_id = leaves.into_iter().next();
        }
    }

    pub fn clear_search(&mut self) {
        if let Self::Loaded {
            selected_id,
            search,
            rows,
            collapsed,
            ..
        } = self
        {
            search.clear();
            let leaves = Self::leaf_profile_ids(rows, search, collapsed);
            *selected_id = leaves.into_iter().next();
        }
    }

    pub fn toggle_group(&mut self, group: &str) {
        if let Self::Loaded { collapsed, .. } = self {
            if let Some(index) = collapsed.iter().position(|name| name == group) {
                collapsed.remove(index);
            } else {
                collapsed.push(group.to_owned());
            }
        }
    }

    pub fn set_selected_id(&mut self, id: Option<String>) {
        if let Self::Loaded { selected_id, .. } = self {
            *selected_id = id;
        }
    }

    #[must_use]
    pub fn is_group_collapsed(&self, group: &str) -> bool {
        match self {
            Self::Loaded { collapsed, .. } => collapsed.iter().any(|name| name == group),
            _ => false,
        }
    }

    /// Flat leaf profile ids in tree display order (expanded groups only).
    fn leaf_profile_ids(
        rows: &[ProfileRowProjection],
        search: &str,
        collapsed: &[String],
    ) -> Vec<String> {
        let visible = Self::filter_rows(rows, search);
        let mut groups: Vec<Option<String>> = Vec::new();
        for row in &visible {
            let key = row.group.clone();
            if !groups.iter().any(|existing| existing == &key) {
                groups.push(key);
            }
        }
        groups.sort_by(|left, right| match (left, right) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(a), Some(b)) => a.cmp(b),
        });
        let mut out = Vec::new();
        for group in groups {
            let name = group.as_deref().unwrap_or("");
            let is_collapsed = group
                .as_ref()
                .is_some_and(|g| collapsed.iter().any(|c| c == g));
            if group.is_some() && is_collapsed {
                continue;
            }
            for row in &visible {
                let row_group = row.group.as_deref().unwrap_or("");
                if row_group == name {
                    out.push(row.id_hex.clone());
                }
            }
        }
        out
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
