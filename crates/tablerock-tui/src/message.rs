//! Facts and semantic intents accepted by the root reducer.

use std::fmt;

use crate::{ScrollDirection, ShellGeometry, ShellTarget};

pub const MAX_PASTE_BYTES: usize = 1_048_576;

#[derive(Clone, PartialEq, Eq)]
pub struct PasteText {
    text: String,
    truncated: bool,
}

impl PasteText {
    #[must_use]
    pub fn bounded(mut text: String) -> Self {
        let mut truncated = text.len() > MAX_PASTE_BYTES;
        if truncated {
            let mut boundary = MAX_PASTE_BYTES;
            while !text.is_char_boundary(boundary) {
                boundary -= 1;
            }
            text.truncate(boundary);
            truncated = true;
        }
        Self { text, truncated }
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub const fn was_truncated(&self) -> bool {
        self.truncated
    }
}

impl fmt::Debug for PasteText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PasteText")
            .field("bytes", &self.text.len())
            .field("truncated", &self.truncated)
            .finish()
    }
}

use crate::model::profiles::{FailureProjection, ProfileRowProjection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfilesMsg {
    ListLoaded {
        request_token: u64,
        items: Vec<ProfileRowProjection>,
    },
    ListFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    Saved {
        request_token: u64,
    },
    SaveFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    Deleted {
        request_token: u64,
    },
    DeleteFailed {
        request_token: u64,
        reason: FailureProjection,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineMsg {
    HealthOk {
        request_token: u64,
    },
    HealthFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    TestOk {
        request_token: u64,
        identity: String,
        elapsed_millis: u64,
    },
    TestFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    ConnectOk {
        request_token: u64,
        session_id_hex: String,
        identity: String,
        temporary: bool,
        engine_label: String,
    },
    ConnectFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    DisconnectOk {
        request_token: u64,
        session_id_hex: String,
    },
    DisconnectFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    /// Prompt-on-connect required; no network I/O happened yet.
    PasswordPromptRequired {
        request_token: u64,
        profile_id_hex: String,
    },
    Reconnecting {
        request_token: u64,
        attempt: u32,
        next_delay_ms: u64,
    },
    ReconnectStopped {
        request_token: u64,
        reason: FailureProjection,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Resize {
        width: u16,
        height: u16,
    },
    FrameRendered(ShellGeometry),
    TerminalFocusChanged(bool),
    Paste(PasteText),
    PointerHovered(Option<ShellTarget>),
    PointerPressed(Option<ShellTarget>),
    PointerDragged(Option<ShellTarget>),
    PointerReleased(Option<ShellTarget>),
    PointerScrolled {
        target: Option<ShellTarget>,
        direction: ScrollDirection,
    },
    EngineResyncRequired,
    EngineResynchronized,
    Profiles(ProfilesMsg),
    Engine(EngineMsg),
    FocusNext,
    FocusPrevious,
    ActionNext,
    ActionPrevious,
    Activate,
    RequestRedraw,
    Quit,
}
