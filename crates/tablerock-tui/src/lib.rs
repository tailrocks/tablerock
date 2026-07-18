//! TableRock's root terminal application.

pub mod effect;
pub mod keymap;
pub mod message;
pub mod model;
pub mod subscriptions;
pub mod update;
pub mod view;

pub use effect::{Effect, EngineKind, ProfileListFilterSpec, ProfileRef, RequestToken};
pub use keymap::{ShellKeyAction, default_keymap};
pub use message::{EngineMsg, MAX_PASTE_BYTES, Message, PasteText, ProfilesMsg};
pub use model::{
    ActionId, FocusRegion, LayoutMode, Model, Screen, ScrollDirection, ShellTarget,
    profiles::{FailureProjection, ProfileListState, ProfileRowProjection},
};
pub use update::{Update, update};
pub use view::{ShellGeometry, ShellView};
