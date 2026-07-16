//! TableRock's root terminal application.

pub mod effect;
pub mod message;
pub mod model;
pub mod subscriptions;
pub mod update;
pub mod view;

pub use effect::Effect;
pub use message::Message;
pub use model::{ActionId, FocusRegion, LayoutMode, Model, Screen};
pub use update::update;
pub use view::ShellView;
