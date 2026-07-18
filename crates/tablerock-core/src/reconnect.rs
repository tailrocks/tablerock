//! Shared bounded reconnect policy for every client.

use crate::ReconnectPreference;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectDecision {
    Manual,
    StopAuthentication,
    RetryAfter { delay_millis: u64 },
    Exhausted,
}

/// Decides one zero-based reconnect attempt. Attempt zero is immediate; later
/// attempts use capped exponential backoff. Total delayed time is at most 61s.
#[must_use]
pub const fn reconnect_decision(
    preference: ReconnectPreference,
    attempt: u32,
    authentication_failed: bool,
) -> ReconnectDecision {
    if matches!(preference, ReconnectPreference::Manual) {
        return ReconnectDecision::Manual;
    }
    if authentication_failed {
        return ReconnectDecision::StopAuthentication;
    }
    match attempt {
        0 => ReconnectDecision::RetryAfter { delay_millis: 0 },
        1 => ReconnectDecision::RetryAfter {
            delay_millis: 1_000,
        },
        2 => ReconnectDecision::RetryAfter {
            delay_millis: 2_000,
        },
        3 => ReconnectDecision::RetryAfter {
            delay_millis: 4_000,
        },
        4 => ReconnectDecision::RetryAfter {
            delay_millis: 8_000,
        },
        5 => ReconnectDecision::RetryAfter {
            delay_millis: 16_000,
        },
        6 => ReconnectDecision::RetryAfter {
            delay_millis: 30_000,
        },
        _ => ReconnectDecision::Exhausted,
    }
}

/// Compatibility classifier for redacted adapters that have not yet adopted
/// structured failure classes. Never inspect raw server text at presentation.
#[must_use]
pub fn reconnect_stops_for_redacted_label(label: &str) -> bool {
    let lower = label.to_ascii_lowercase();
    lower.contains("auth") || lower.contains("password prompt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_policy_is_immediate_then_bounded() {
        assert_eq!(
            reconnect_decision(ReconnectPreference::BoundedAutomatic, 0, false),
            ReconnectDecision::RetryAfter { delay_millis: 0 }
        );
        assert_eq!(
            reconnect_decision(ReconnectPreference::BoundedAutomatic, 6, false),
            ReconnectDecision::RetryAfter {
                delay_millis: 30_000
            }
        );
        assert_eq!(
            reconnect_decision(ReconnectPreference::BoundedAutomatic, 7, false),
            ReconnectDecision::Exhausted
        );
        let total = (0..=6)
            .map(|attempt| {
                match reconnect_decision(ReconnectPreference::BoundedAutomatic, attempt, false) {
                    ReconnectDecision::RetryAfter { delay_millis } => delay_millis,
                    _ => 0,
                }
            })
            .sum::<u64>();
        assert_eq!(total, 61_000);
    }

    #[test]
    fn manual_and_authentication_never_retry() {
        assert_eq!(
            reconnect_decision(ReconnectPreference::Manual, 0, false),
            ReconnectDecision::Manual
        );
        assert_eq!(
            reconnect_decision(ReconnectPreference::BoundedAutomatic, 0, true),
            ReconnectDecision::StopAuthentication
        );
        assert!(reconnect_stops_for_redacted_label("authentication"));
        assert!(reconnect_stops_for_redacted_label("password prompt failed"));
        assert!(!reconnect_stops_for_redacted_label("connection"));
    }
}
