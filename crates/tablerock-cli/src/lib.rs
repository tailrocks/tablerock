//! Process and terminal adapters for TableRock.

mod input;
mod run;

pub use crossterm::event::EventStream;
pub use input::{InputAdapter, map_event};
pub use run::{
    RootMessageReceiver, RootMessageSender, RunError, root_message_channel, run, run_caught,
    run_with_root_messages,
};
pub use termrock::crossterm::{Session, SessionOptions};
