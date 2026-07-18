//! Process and terminal adapters for TableRock.

mod effects;
mod ingress;
mod input;
mod projection;
mod run;

#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;

pub use crossterm::event::EventStream;
pub use ingress::{
    Delivery, IngressReceiver, IngressSender, SendOutcome, TryReceiveError, bounded_ingress,
};
pub use input::{InputAdapter, map_backend_event, map_event};
pub use run::{
    RootMessageReceiver, RootMessageSender, RootProgress, RunError, root_message_channel, run,
    run_caught, run_with_root_messages,
};
pub use termrock::crossterm::{Session, SessionOptions};
