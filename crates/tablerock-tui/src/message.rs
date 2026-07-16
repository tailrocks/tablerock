//! Facts and semantic intents accepted by the root reducer.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    Resize { width: u16, height: u16 },
    FocusNext,
    FocusPrevious,
    ActionNext,
    ActionPrevious,
    Activate,
    RequestRedraw,
    Quit,
}
