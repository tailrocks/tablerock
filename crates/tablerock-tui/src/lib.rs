//! TableRock's root terminal application.

pub mod effect;
pub mod keymap;
pub mod message;
pub mod model;
pub mod subscriptions;
pub mod update;
pub mod view;

pub use effect::Effect;
pub use keymap::{ShellKeyAction, default_keymap};
pub use message::{MAX_PASTE_BYTES, Message, PasteText};
pub use model::{ActionId, FocusRegion, LayoutMode, Model, Screen, ScrollDirection, ShellTarget};
pub use update::{Update, update};
pub use view::{ShellGeometry, ShellView};
