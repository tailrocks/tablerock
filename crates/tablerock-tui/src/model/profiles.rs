//! Connections-screen profile list submodel.

use crate::effect::RequestToken;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRowProjection {
    pub name: String,
    pub engine_label: String,
    pub group: Option<String>,
    pub favorite: bool,
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
