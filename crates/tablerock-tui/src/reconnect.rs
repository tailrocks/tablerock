//! Bounded reconnect backoff policy (no clocks in the reducer; delays apply in executor).

/// Next delay in milliseconds for reconnect attempt `attempt` (0-based).
/// Returns `None` when the backoff budget is exhausted.
#[must_use]
pub const fn next_backoff_ms(attempt: u32) -> Option<u64> {
    match attempt {
        0 => Some(1_000),
        1 => Some(2_000),
        2 => Some(4_000),
        3 => Some(8_000),
        4 => Some(16_000),
        5 => Some(30_000),
        _ => None,
    }
}

/// Authentication failures must stop reconnect; other classes may retry.
#[must_use]
pub fn stop_on_failure_label(label: &str) -> bool {
    // Redacted labels from adapters; match known authentication markers only.
    let lower = label.to_ascii_lowercase();
    lower.contains("auth") || lower.contains("password prompt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_is_bounded_and_capped() {
        assert_eq!(next_backoff_ms(0), Some(1_000));
        assert_eq!(next_backoff_ms(5), Some(30_000));
        assert_eq!(next_backoff_ms(6), None);
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
