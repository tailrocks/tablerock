use std::{error::Error, fmt, panic::AssertUnwindSafe};

/// Typed bridge failures. Messages never include SQL, credentials, or cell values.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum BridgeError {
    /// Recoverable rejection (validation, capacity, unknown id, stale revision).
    Rejected { code: String, message: String },
    /// A Rust panic was caught at the facade boundary; process remains usable.
    ContainedPanic { message: String },
    /// Runtime already stopped or not yet constructed.
    RuntimeUnavailable,
    /// Requested session is not registered.
    UnknownSession,
    /// Requested operation is not registered.
    UnknownOperation,
    /// Requested page is not resident.
    UnknownPage,
    /// Event cursor is in the future of the producer.
    FutureCursor,
    /// Event cursor requires resync (gap or retired producer).
    ResyncRequired,
    /// Facade has begun or completed shutdown.
    ShuttingDown,
}

impl BridgeError {
    #[must_use]
    pub fn rejected(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Rejected {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for BridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected { code, message } => write!(formatter, "{code}: {message}"),
            Self::ContainedPanic { message } => {
                write!(formatter, "contained panic: {message}")
            }
            Self::RuntimeUnavailable => formatter.write_str("runtime unavailable"),
            Self::UnknownSession => formatter.write_str("unknown session"),
            Self::UnknownOperation => formatter.write_str("unknown operation"),
            Self::UnknownPage => formatter.write_str("unknown page"),
            Self::FutureCursor => formatter.write_str("event cursor is in the future"),
            Self::ResyncRequired => formatter.write_str("event cursor requires resync"),
            Self::ShuttingDown => formatter.write_str("bridge is shutting down"),
        }
    }
}

impl Error for BridgeError {}

/// Run `body` and convert panics into [`BridgeError::ContainedPanic`].
pub(crate) fn catch_entry<T>(
    body: impl FnOnce() -> Result<T, BridgeError>,
) -> Result<T, BridgeError> {
    match std::panic::catch_unwind(AssertUnwindSafe(body)) {
        Ok(result) => result,
        Err(payload) => Err(BridgeError::ContainedPanic {
            message: panic_message(payload),
        }),
    }
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_owned();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "non-string panic payload".to_owned()
}
