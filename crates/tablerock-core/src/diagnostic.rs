use std::{error::Error, fmt};

use crate::Engine;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostgreSqlCode {
    UniqueViolation,
    ForeignKeyViolation,
    SerializationFailure,
    DeadlockDetected,
    AdminShutdown,
    QueryCanceled,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedisCode {
    Error,
    WrongType,
    NoAuth,
    Moved,
    Ask,
    Busy,
    NoScript,
    ReadOnly,
    MasterDown,
    Misconfigured,
    OutOfMemory,
    ExecAbort,
    Loading,
    ClusterDown,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApplicationCode {
    Timeout,
    ResourceLimit,
    Unsupported,
    StaleRevision,
    SafetyRejected,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SafeCode {
    PostgreSql(PostgreSqlCode),
    ClickHouse(u32),
    Redis(RedisCode),
    Application(ApplicationCode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureClass {
    Authentication,
    Authorization,
    Connectivity,
    Timeout,
    InvalidInput,
    Conflict,
    SafetyRejected,
    Unsupported,
    ResourceLimit,
    Server,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Information,
    Warning,
    Error,
    Fatal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PositionUnit {
    ServerCharacter,
    ByteOffset,
    ArgumentIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiagnosticPosition {
    unit: PositionUnit,
    value: u64,
}

impl DiagnosticPosition {
    #[must_use]
    pub const fn new(unit: PositionUnit, value: u64) -> Self {
        Self { unit, value }
    }

    #[must_use]
    pub const fn unit(self) -> PositionUnit {
        self.unit
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperatorAction {
    None,
    ReviewInput,
    ReviewOutcome,
    Reauthenticate,
    Reconnect,
    ReduceScope,
    UpgradeServer,
    ReportBug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutcomeCertainty {
    NotDispatched,
    ReadOnly,
    WriteNotApplied,
    WriteApplied,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationSafety {
    ProvenReadOnly,
    MayWrite,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RetryAdvice {
    Never,
    AfterUserAction,
    ExplicitRequest,
    SafeAutomaticReadOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SafeDiagnostic {
    class: FailureClass,
    engine: Engine,
    code: Option<SafeCode>,
    severity: Severity,
    position: Option<DiagnosticPosition>,
    action: OperatorAction,
    certainty: OutcomeCertainty,
    safety: OperationSafety,
    retry: RetryAdvice,
}

impl SafeDiagnostic {
    #[must_use]
    pub const fn new(
        class: FailureClass,
        engine: Engine,
        severity: Severity,
        certainty: OutcomeCertainty,
        safety: OperationSafety,
    ) -> Self {
        Self {
            class,
            engine,
            code: None,
            severity,
            position: None,
            action: OperatorAction::None,
            certainty,
            safety,
            retry: RetryAdvice::Never,
        }
    }

    pub fn with_code(mut self, code: SafeCode) -> Result<Self, DiagnosticBuildError> {
        let matches_engine = matches!(code, SafeCode::Application(_))
            || matches!(
                (self.engine, code),
                (Engine::PostgreSql, SafeCode::PostgreSql(_))
                    | (Engine::ClickHouse, SafeCode::ClickHouse(_))
                    | (Engine::Redis, SafeCode::Redis(_))
            );
        if !matches_engine {
            return Err(DiagnosticBuildError::CodeEngineMismatch {
                diagnostic_engine: self.engine,
            });
        }
        self.code = Some(code);
        Ok(self)
    }

    #[must_use]
    pub const fn with_position(mut self, position: DiagnosticPosition) -> Self {
        self.position = Some(position);
        self
    }

    #[must_use]
    pub const fn with_action(mut self, action: OperatorAction) -> Self {
        self.action = action;
        self
    }

    pub fn with_retry(mut self, retry: RetryAdvice) -> Result<Self, DiagnosticBuildError> {
        if matches!(retry, RetryAdvice::SafeAutomaticReadOnly)
            && (!matches!(self.safety, OperationSafety::ProvenReadOnly)
                || !matches!(
                    self.certainty,
                    OutcomeCertainty::NotDispatched | OutcomeCertainty::ReadOnly
                ))
        {
            return Err(DiagnosticBuildError::UnsafeRetryAdvice {
                certainty: self.certainty,
                safety: self.safety,
            });
        }
        self.retry = retry;
        Ok(self)
    }

    #[must_use]
    pub const fn class(&self) -> FailureClass {
        self.class
    }
    #[must_use]
    pub const fn engine(&self) -> Engine {
        self.engine
    }
    #[must_use]
    pub const fn code(&self) -> Option<SafeCode> {
        self.code
    }
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }
    #[must_use]
    pub const fn position(&self) -> Option<DiagnosticPosition> {
        self.position
    }
    #[must_use]
    pub const fn action(&self) -> OperatorAction {
        self.action
    }
    #[must_use]
    pub const fn certainty(&self) -> OutcomeCertainty {
        self.certainty
    }
    #[must_use]
    pub const fn safety(&self) -> OperationSafety {
        self.safety
    }
    #[must_use]
    pub const fn retry(&self) -> RetryAdvice {
        self.retry
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticBuildError {
    CodeEngineMismatch {
        diagnostic_engine: Engine,
    },
    UnsafeRetryAdvice {
        certainty: OutcomeCertainty,
        safety: OperationSafety,
    },
}

impl fmt::Display for DiagnosticBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CodeEngineMismatch { .. } => {
                "safe diagnostic code does not belong to the diagnostic engine"
            }
            Self::UnsafeRetryAdvice { .. } => {
                "automatic retry requires proven read-only operation safety"
            }
        })
    }
}

impl Error for DiagnosticBuildError {}
