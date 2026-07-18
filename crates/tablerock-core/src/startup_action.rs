//! Reviewed, bounded startup SQL/commands per profile.
//!
//! Execution is engine-owned; this module defines the durable contract:
//! safety class, timeout, reconnect policy, and ordered set bounds.

use std::{error::Error, fmt};

use crate::{BoundedText, ByteLimit};

/// Maximum startup actions per profile.
pub const MAX_STARTUP_ACTIONS: usize = 16;

/// Maximum statement bytes for a single startup action.
pub const MAX_STARTUP_STATEMENT_BYTES: u64 = 8_192;

/// Safety classification for a startup statement (review gate uses this).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StartupSafetyClass {
    /// SELECT / SHOW / Redis read — may auto-run when profile allows.
    ReadOnly,
    /// INSERT/UPDATE/DDL-like — requires ConfirmWrites + review.
    Write,
    /// DROP/TRUNCATE/FLUSHALL class — never auto-run; explicit review only.
    Dangerous,
}

impl StartupSafetyClass {
    /// Whether this class may auto-execute without an extra review handle.
    #[must_use]
    pub const fn may_auto_run(self) -> bool {
        matches!(self, Self::ReadOnly)
    }
}

/// One bounded startup statement with timeout and reconnect policy.
#[derive(Clone, PartialEq, Eq)]
pub struct StartupAction {
    statement: BoundedText,
    safety: StartupSafetyClass,
    timeout_ms: u32,
    run_on_reconnect: bool,
}

impl StartupAction {
    pub const MIN_TIMEOUT_MS: u32 = 100;
    pub const MAX_TIMEOUT_MS: u32 = 120_000;

    pub fn new(
        statement: BoundedText,
        safety: StartupSafetyClass,
        timeout_ms: u32,
        run_on_reconnect: bool,
    ) -> Result<Self, StartupActionError> {
        if statement.is_empty() {
            return Err(StartupActionError::EmptyStatement);
        }
        if statement.len() as u64 > MAX_STARTUP_STATEMENT_BYTES {
            return Err(StartupActionError::StatementTooLarge {
                actual: statement.len() as u64,
                maximum: MAX_STARTUP_STATEMENT_BYTES,
            });
        }
        if !(Self::MIN_TIMEOUT_MS..=Self::MAX_TIMEOUT_MS).contains(&timeout_ms) {
            return Err(StartupActionError::InvalidTimeout { timeout_ms });
        }
        Ok(Self {
            statement,
            safety,
            timeout_ms,
            run_on_reconnect,
        })
    }

    /// Convenience constructor that copies and bounds the statement text.
    pub fn from_str(
        statement: &str,
        safety: StartupSafetyClass,
        timeout_ms: u32,
        run_on_reconnect: bool,
    ) -> Result<Self, StartupActionError> {
        let bounded =
            BoundedText::copy_from_str(statement, ByteLimit::new(MAX_STARTUP_STATEMENT_BYTES))
                .map_err(|_| StartupActionError::StatementTooLarge {
                    actual: statement.len() as u64,
                    maximum: MAX_STARTUP_STATEMENT_BYTES,
                })?;
        Self::new(bounded, safety, timeout_ms, run_on_reconnect)
    }

    #[must_use]
    pub fn statement(&self) -> &str {
        self.statement.as_str()
    }

    #[must_use]
    pub const fn safety(&self) -> StartupSafetyClass {
        self.safety
    }

    #[must_use]
    pub const fn timeout_ms(&self) -> u32 {
        self.timeout_ms
    }

    #[must_use]
    pub const fn run_on_reconnect(&self) -> bool {
        self.run_on_reconnect
    }
}

impl fmt::Debug for StartupAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StartupAction")
            .field("statement_bytes", &self.statement.len())
            .field("safety", &self.safety)
            .field("timeout_ms", &self.timeout_ms)
            .field("run_on_reconnect", &self.run_on_reconnect)
            .finish()
    }
}

/// Ordered set of startup actions for one profile.
#[derive(Clone, PartialEq, Eq)]
pub struct StartupActionSet {
    actions: Vec<StartupAction>,
}

impl StartupActionSet {
    pub fn new(actions: Vec<StartupAction>) -> Result<Self, StartupActionError> {
        if actions.len() > MAX_STARTUP_ACTIONS {
            return Err(StartupActionError::TooManyActions {
                actual: actions.len(),
                maximum: MAX_STARTUP_ACTIONS,
            });
        }
        Ok(Self { actions })
    }

    #[must_use]
    pub fn empty() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    #[must_use]
    pub fn actions(&self) -> &[StartupAction] {
        &self.actions
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Actions that should run on this connect (initial vs reconnect).
    #[must_use]
    pub fn for_connect_event(&self, is_reconnect: bool) -> Vec<&StartupAction> {
        self.actions
            .iter()
            .filter(|action| !is_reconnect || action.run_on_reconnect())
            .collect()
    }

    /// Actions that may auto-run without an additional review handle.
    #[must_use]
    pub fn auto_runnable(&self, is_reconnect: bool) -> Vec<&StartupAction> {
        self.for_connect_event(is_reconnect)
            .into_iter()
            .filter(|action| action.safety().may_auto_run())
            .collect()
    }

    /// Actions that require review before execution.
    #[must_use]
    pub fn review_required(&self, is_reconnect: bool) -> Vec<&StartupAction> {
        self.for_connect_event(is_reconnect)
            .into_iter()
            .filter(|action| !action.safety().may_auto_run())
            .collect()
    }
}

impl fmt::Debug for StartupActionSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StartupActionSet")
            .field("len", &self.actions.len())
            .finish_non_exhaustive()
    }
}

/// Outcome of one startup action attempt (engine maps driver results here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupActionOutcome {
    Succeeded,
    TimedOut,
    Failed,
    SkippedNeedsReview,
    Cancelled,
}

/// Aggregate run result for a connect/reconnect batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupRunReport {
    outcomes: Vec<(usize, StartupActionOutcome)>,
}

impl StartupRunReport {
    #[must_use]
    pub fn new(outcomes: Vec<(usize, StartupActionOutcome)>) -> Self {
        Self { outcomes }
    }

    #[must_use]
    pub fn outcomes(&self) -> &[(usize, StartupActionOutcome)] {
        &self.outcomes
    }

    #[must_use]
    pub fn has_failure(&self) -> bool {
        self.outcomes.iter().any(|(_, o)| {
            matches!(
                o,
                StartupActionOutcome::Failed | StartupActionOutcome::TimedOut
            )
        })
    }

    #[must_use]
    pub fn all_succeeded_or_skipped(&self) -> bool {
        self.outcomes.iter().all(|(_, o)| {
            matches!(
                o,
                StartupActionOutcome::Succeeded | StartupActionOutcome::SkippedNeedsReview
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupActionError {
    EmptyStatement,
    StatementTooLarge { actual: u64, maximum: u64 },
    InvalidTimeout { timeout_ms: u32 },
    TooManyActions { actual: usize, maximum: usize },
}

impl fmt::Display for StartupActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyStatement => f.write_str("startup statement must not be empty"),
            Self::StatementTooLarge { actual, maximum } => {
                write!(f, "startup statement {actual} bytes exceeds max {maximum}")
            }
            Self::InvalidTimeout { timeout_ms } => {
                write!(f, "startup timeout_ms {timeout_ms} out of range")
            }
            Self::TooManyActions { actual, maximum } => {
                write!(f, "startup action count {actual} exceeds max {maximum}")
            }
        }
    }
}

impl Error for StartupActionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_and_oversized_statement() {
        assert!(matches!(
            StartupAction::from_str("", StartupSafetyClass::ReadOnly, 1_000, false),
            Err(StartupActionError::EmptyStatement)
        ));
        let big = "x".repeat((MAX_STARTUP_STATEMENT_BYTES as usize) + 1);
        assert!(matches!(
            StartupAction::from_str(&big, StartupSafetyClass::ReadOnly, 1_000, false),
            Err(StartupActionError::StatementTooLarge { .. })
        ));
    }

    #[test]
    fn timeout_bounds() {
        assert!(
            StartupAction::from_str("SELECT 1", StartupSafetyClass::ReadOnly, 50, false).is_err()
        );
        assert!(
            StartupAction::from_str("SELECT 1", StartupSafetyClass::ReadOnly, 200_000, false)
                .is_err()
        );
        assert!(
            StartupAction::from_str("SELECT 1", StartupSafetyClass::ReadOnly, 5_000, true).is_ok()
        );
    }

    #[test]
    fn set_filters_reconnect_and_review() {
        let set = StartupActionSet::new(vec![
            StartupAction::from_str("SELECT 1", StartupSafetyClass::ReadOnly, 1_000, true).unwrap(),
            StartupAction::from_str(
                "INSERT INTO t VALUES (1)",
                StartupSafetyClass::Write,
                2_000,
                false,
            )
            .unwrap(),
            StartupAction::from_str("DROP TABLE t", StartupSafetyClass::Dangerous, 3_000, true)
                .unwrap(),
        ])
        .unwrap();

        assert_eq!(set.for_connect_event(false).len(), 3);
        assert_eq!(set.for_connect_event(true).len(), 2); // reconnect drops non-reconnect insert
        assert_eq!(set.auto_runnable(false).len(), 1);
        assert_eq!(set.review_required(false).len(), 2);
        assert_eq!(set.auto_runnable(true).len(), 1);
        assert_eq!(set.review_required(true).len(), 1); // dangerous only
    }

    #[test]
    fn set_caps_action_count() {
        let actions: Vec<_> = (0..MAX_STARTUP_ACTIONS + 1)
            .map(|i| {
                StartupAction::from_str(
                    &format!("SELECT {i}"),
                    StartupSafetyClass::ReadOnly,
                    1_000,
                    false,
                )
                .unwrap()
            })
            .collect();
        assert!(matches!(
            StartupActionSet::new(actions),
            Err(StartupActionError::TooManyActions { .. })
        ));
    }

    #[test]
    fn debug_redacts_statement_text() {
        let action = StartupAction::from_str(
            "SELECT secret_token FROM t",
            StartupSafetyClass::ReadOnly,
            1_000,
            false,
        )
        .unwrap();
        let debug = format!("{action:?}");
        assert!(!debug.contains("secret_token"));
        assert!(debug.contains("statement_bytes"));
    }

    #[test]
    fn report_partial_failure() {
        let report = StartupRunReport::new(vec![
            (0, StartupActionOutcome::Succeeded),
            (1, StartupActionOutcome::Failed),
            (2, StartupActionOutcome::SkippedNeedsReview),
        ]);
        assert!(report.has_failure());
        assert!(!report.all_succeeded_or_skipped());
    }
}
