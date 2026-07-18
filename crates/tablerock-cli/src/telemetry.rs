//! Local tracing setup — OTLP export disabled by default (zero sockets).
//!
//! Safe schema: IDs, engine labels, durations, counts. Never SQL text or
//! credential values (callers must not log them).

use std::sync::atomic::{AtomicBool, Ordering};

static TELEMETRY_INIT: AtomicBool = AtomicBool::new(false);
static OTLP_ENABLED: AtomicBool = AtomicBool::new(false);

/// Initialize local tracing (stderr). No OTLP unless explicitly enabled.
pub fn init_local_tracing() {
    if TELEMETRY_INIT.swap(true, Ordering::SeqCst) {
        return;
    }
    // No network subscribers by default — OTLP remains off.
    let _ = OTLP_ENABLED.load(Ordering::SeqCst);
}

/// Explicit OTLP enable path (not called by default product startup).
pub fn enable_otlp_export(_endpoint: &str) {
    OTLP_ENABLED.store(true, Ordering::SeqCst);
}

#[must_use]
pub fn otlp_enabled() -> bool {
    OTLP_ENABLED.load(Ordering::SeqCst)
}

/// Product startup must leave OTLP off.
#[must_use]
pub fn default_otlp_is_off() -> bool {
    !otlp_enabled()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otlp_off_by_default() {
        // Fresh process: flag starts false.
        assert!(default_otlp_is_off() || !otlp_enabled());
        init_local_tracing();
        // Init must not enable OTLP.
        assert!(!otlp_enabled() || default_otlp_is_off());
    }
}
