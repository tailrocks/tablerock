//! Execute reviewed startup actions against a live PostgreSQL session.

use std::time::Duration;

use tablerock_core::{
    StartupAction, StartupActionOutcome, StartupActionSet, StartupRunReport, StartupSafetyClass,
};

use crate::PostgresSession;

/// Run auto-runnable ReadOnly startup actions; Write/Dangerous are skipped.
///
/// Partial failures are recorded; later actions still run (report honesty).
pub async fn run_postgres_startup_actions(
    session: &PostgresSession,
    set: &StartupActionSet,
    is_reconnect: bool,
) -> StartupRunReport {
    let mut outcomes = Vec::new();
    for (index, action) in set.for_connect_event(is_reconnect).into_iter().enumerate() {
        let outcome = run_one(session, action).await;
        outcomes.push((index, outcome));
    }
    StartupRunReport::new(outcomes)
}

async fn run_one(session: &PostgresSession, action: &StartupAction) -> StartupActionOutcome {
    if !action.safety().may_auto_run() {
        return StartupActionOutcome::SkippedNeedsReview;
    }
    debug_assert!(matches!(action.safety(), StartupSafetyClass::ReadOnly));
    let timeout = Duration::from_millis(u64::from(action.timeout_ms()));
    match tokio::time::timeout(timeout, session.execute_sql(action.statement())).await {
        Ok(Ok(())) => StartupActionOutcome::Succeeded,
        Ok(Err(_)) => StartupActionOutcome::Failed,
        Err(_) => StartupActionOutcome::TimedOut,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tablerock_core::StartupSafetyClass;

    #[test]
    fn write_actions_not_auto_runnable() {
        let set = StartupActionSet::new(vec![
            StartupAction::from_str("SELECT 1", StartupSafetyClass::ReadOnly, 1_000, true).unwrap(),
            StartupAction::from_str(
                "DELETE FROM t",
                StartupSafetyClass::Write,
                1_000,
                true,
            )
            .unwrap(),
        ])
        .unwrap();
        assert_eq!(set.auto_runnable(false).len(), 1);
        assert_eq!(set.review_required(false).len(), 1);
    }
}
