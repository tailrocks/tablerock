//! Execute reviewed startup actions against live engine sessions.

use std::time::Duration;

use tablerock_core::{
    StartupAction, StartupActionOutcome, StartupActionSet, StartupRunReport, StartupSafetyClass,
};

use crate::{ClickHouseSession, PostgresSession, RedisSession};

/// Run auto-runnable ReadOnly startup SQL against PostgreSQL.
///
/// Write/Dangerous → `SkippedNeedsReview`. Partial failures continue.
pub async fn run_postgres_startup_actions(
    session: &PostgresSession,
    set: &StartupActionSet,
    is_reconnect: bool,
) -> StartupRunReport {
    let mut outcomes = Vec::new();
    for (index, action) in set.for_connect_event(is_reconnect).into_iter().enumerate() {
        outcomes.push((index, run_sql_one_pg(session, action).await));
    }
    StartupRunReport::new(outcomes)
}

/// Run auto-runnable ReadOnly startup SQL against ClickHouse.
pub async fn run_clickhouse_startup_actions(
    session: &ClickHouseSession,
    set: &StartupActionSet,
    is_reconnect: bool,
) -> StartupRunReport {
    let mut outcomes = Vec::new();
    for (index, action) in set.for_connect_event(is_reconnect).into_iter().enumerate() {
        outcomes.push((index, run_sql_one_ch(session, action).await));
    }
    StartupRunReport::new(outcomes)
}

/// Run auto-runnable ReadOnly startup commands against Redis.
///
/// Statement text is whitespace-tokenized: first token = command name, rest = args.
pub async fn run_redis_startup_actions(
    session: &RedisSession,
    set: &StartupActionSet,
    is_reconnect: bool,
) -> StartupRunReport {
    let mut outcomes = Vec::new();
    for (index, action) in set.for_connect_event(is_reconnect).into_iter().enumerate() {
        outcomes.push((index, run_redis_one(session, action).await));
    }
    StartupRunReport::new(outcomes)
}

async fn run_sql_one_pg(session: &PostgresSession, action: &StartupAction) -> StartupActionOutcome {
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

async fn run_sql_one_ch(
    session: &ClickHouseSession,
    action: &StartupAction,
) -> StartupActionOutcome {
    if !action.safety().may_auto_run() {
        return StartupActionOutcome::SkippedNeedsReview;
    }
    let timeout = Duration::from_millis(u64::from(action.timeout_ms()));
    match tokio::time::timeout(timeout, session.execute_sql(action.statement())).await {
        Ok(Ok(())) => StartupActionOutcome::Succeeded,
        Ok(Err(_)) => StartupActionOutcome::Failed,
        Err(_) => StartupActionOutcome::TimedOut,
    }
}

async fn run_redis_one(session: &RedisSession, action: &StartupAction) -> StartupActionOutcome {
    if !action.safety().may_auto_run() {
        return StartupActionOutcome::SkippedNeedsReview;
    }
    let mut parts = action.statement().split_whitespace();
    let Some(name) = parts.next() else {
        return StartupActionOutcome::Failed;
    };
    let args: Vec<Vec<u8>> = parts.map(|p| p.as_bytes().to_vec()).collect();
    let timeout = Duration::from_millis(u64::from(action.timeout_ms()));
    match tokio::time::timeout(
        timeout,
        session.execute_command_argv(&name.to_ascii_uppercase(), &args),
    )
    .await
    {
        Ok(Ok(_)) => StartupActionOutcome::Succeeded,
        Ok(Err(_)) => StartupActionOutcome::Failed,
        Err(_) => StartupActionOutcome::TimedOut,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
