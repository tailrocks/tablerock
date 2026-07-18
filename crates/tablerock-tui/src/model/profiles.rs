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
            Self::Loaded { rows, .. } => format!("Profiles: {}", rows.len()),
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
}
