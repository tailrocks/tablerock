//! Bounded input sources merged by the CLI adapter.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subscription {
    TerminalInput,
    EngineEvents { capacity: usize },
    Signals,
}

pub const ENGINE_EVENT_CAPACITY: usize = 256;

#[must_use]
pub const fn root_subscriptions() -> [Subscription; 3] {
    [
        Subscription::TerminalInput,
        Subscription::EngineEvents {
            capacity: ENGINE_EVENT_CAPACITY,
        },
        Subscription::Signals,
    ]
}
