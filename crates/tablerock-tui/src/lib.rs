//! TableRock's root terminal application.

pub mod effect;
pub mod keymap;
pub mod message;
pub mod model;
pub mod subscriptions;
pub mod update;
pub mod view;

pub use effect::{
    ConnectionDraft, Effect, EngineKind, PasswordSourceSpec, ProfileListFilterSpec, ProfileRef,
    RequestToken, TlsModeSpec,
};
pub use keymap::{ShellKeyAction, default_keymap};
pub use message::{EngineMsg, MAX_PASTE_BYTES, Message, PasteText, ProfilesMsg};
pub use model::{
    ActionId, FocusRegion, LayoutMode, Model, Screen, ScrollDirection, SessionFacts, ShellTarget,
    profiles::{FailureProjection, LiveConnectionState, ProfileListState, ProfileRowProjection},
};
pub use update::{Update, update};
pub use view::{ShellGeometry, ShellView};
