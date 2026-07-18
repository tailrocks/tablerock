//! Bounded reconnect backoff policy (no clocks in the reducer; delays apply in executor).

/// Compatibility projection over the shared core reconnect authority.
#[must_use]
pub const fn next_backoff_ms(attempt: u32) -> Option<u64> {
    match tablerock_core::reconnect_decision(
        tablerock_core::ReconnectPreference::BoundedAutomatic,
        attempt,
        false,
    ) {
        tablerock_core::ReconnectDecision::RetryAfter { delay_millis } => Some(delay_millis),
        _ => None,
    }
}

/// Authentication failures must stop reconnect; other classes may retry.
#[must_use]
pub fn stop_on_failure_label(label: &str) -> bool {
    tablerock_core::reconnect_stops_for_redacted_label(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_is_bounded_and_capped() {
        assert_eq!(next_backoff_ms(0), Some(0));
        assert_eq!(next_backoff_ms(6), Some(30_000));
        assert_eq!(next_backoff_ms(7), None);
        let mut total = 0u64;
        let mut attempt = 0;
        while let Some(delay) = next_backoff_ms(attempt) {
            total += delay;
            attempt += 1;
        }
        assert!(total <= 61_000);
    }

    #[test]
    fn auth_labels_stop_reconnect() {
        assert!(stop_on_failure_label("authentication"));
        assert!(stop_on_failure_label("AdapterFailureClass::Authentication"));
        assert!(!stop_on_failure_label("connection"));
        assert!(!stop_on_failure_label("timeout"));
    }
}
