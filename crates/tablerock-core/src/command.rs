use std::{error::Error, fmt};

use crate::{OperationId, OperationScope, ProfileId, RequestId, ResultId, Revision, SessionId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BudgetField {
    Duration,
    EventCount,
    ResponseBytes,
    PageRows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandBudget {
    max_duration_ms: u64,
    max_event_count: u32,
    max_response_bytes: u64,
    max_page_rows: u32,
}

impl CommandBudget {
    pub fn new(
        max_duration_ms: u64,
        max_event_count: u32,
        max_response_bytes: u64,
        max_page_rows: u32,
    ) -> Result<Self, CommandBudgetError> {
        check_nonzero(max_duration_ms, BudgetField::Duration)?;
        check_nonzero(max_event_count as u64, BudgetField::EventCount)?;
        check_nonzero(max_response_bytes, BudgetField::ResponseBytes)?;
        check_nonzero(max_page_rows as u64, BudgetField::PageRows)?;
        Ok(Self {
            max_duration_ms,
            max_event_count,
            max_response_bytes,
            max_page_rows,
        })
    }

    pub fn validate(
        self,
        limits: CommandBudgetLimits,
    ) -> Result<ValidatedCommandBudget, CommandBudgetError> {
        check_at_most(
            self.max_duration_ms,
            limits.max_duration_ms,
            BudgetField::Duration,
        )?;
        check_at_most(
            self.max_event_count as u64,
            limits.max_event_count as u64,
            BudgetField::EventCount,
        )?;
        check_at_most(
            self.max_response_bytes,
            limits.max_response_bytes,
            BudgetField::ResponseBytes,
        )?;
        check_at_most(
            self.max_page_rows as u64,
            limits.max_page_rows as u64,
            BudgetField::PageRows,
        )?;
        Ok(ValidatedCommandBudget(self))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandBudgetLimits {
    max_duration_ms: u64,
    max_event_count: u32,
    max_response_bytes: u64,
    max_page_rows: u32,
}

impl CommandBudgetLimits {
    pub fn new(
        max_duration_ms: u64,
        max_event_count: u32,
        max_response_bytes: u64,
        max_page_rows: u32,
    ) -> Result<Self, CommandBudgetError> {
        let budget = CommandBudget::new(
            max_duration_ms,
            max_event_count,
            max_response_bytes,
            max_page_rows,
        )?;
        Ok(Self {
            max_duration_ms: budget.max_duration_ms,
            max_event_count: budget.max_event_count,
            max_response_bytes: budget.max_response_bytes,
            max_page_rows: budget.max_page_rows,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValidatedCommandBudget(CommandBudget);

impl ValidatedCommandBudget {
    #[must_use]
    pub const fn max_duration_ms(self) -> u64 {
        self.0.max_duration_ms
    }

    #[must_use]
    pub const fn max_event_count(self) -> u32 {
        self.0.max_event_count
    }

    #[must_use]
    pub const fn max_response_bytes(self) -> u64 {
        self.0.max_response_bytes
    }

    #[must_use]
    pub const fn max_page_rows(self) -> u32 {
        self.0.max_page_rows
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBudgetError {
    ZeroLimit {
        field: BudgetField,
    },
    LimitExceeded {
        field: BudgetField,
        actual: u64,
        limit: u64,
    },
}

impl fmt::Display for CommandBudgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ZeroLimit { .. } => "command budget field must be nonzero",
            Self::LimitExceeded { .. } => "command budget exceeds its owner limit",
        })
    }
}

impl Error for CommandBudgetError {}

const fn check_nonzero(value: u64, field: BudgetField) -> Result<(), CommandBudgetError> {
    if value == 0 {
        Err(CommandBudgetError::ZeroLimit { field })
    } else {
        Ok(())
    }
}

const fn check_at_most(
    actual: u64,
    limit: u64,
    field: BudgetField,
) -> Result<(), CommandBudgetError> {
    if actual > limit {
        Err(CommandBudgetError::LimitExceeded {
            field,
            actual,
            limit,
        })
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandScope {
    Application,
    Profile(ProfileId),
    Session {
        profile_id: ProfileId,
        session_id: SessionId,
    },
    Context(OperationScope),
}

/// Maximum UTF-8 byte length for an operator-supplied statement.
///
/// Matches the paste bound used by the TUI so one statement cannot exceed the
/// largest text payload the shell already admits.
pub const MAX_STATEMENT_BYTES: usize = 1_048_576;

/// Bounded operator statement text. Debug redacts content to length only.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StatementText {
    text: String,
}

impl StatementText {
    pub fn new(text: impl Into<String>) -> Result<Self, StatementTextError> {
        let text = text.into();
        if text.len() > MAX_STATEMENT_BYTES {
            return Err(StatementTextError::TooLarge {
                actual: text.len(),
                limit: MAX_STATEMENT_BYTES,
            });
        }
        Ok(Self { text })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.text.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl fmt::Debug for StatementText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StatementText")
            .field("bytes", &self.text.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementTextError {
    TooLarge { actual: usize, limit: usize },
}

impl fmt::Display for StatementTextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TooLarge { .. } => "statement text exceeds the maximum byte bound",
        })
    }
}

impl Error for StatementTextError {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CommandIntent {
    TestProfile,
    Connect,
    Disconnect,
    RefreshCatalog,
    FetchPage(PageRequest),
    /// Operator-supplied statement. Treated as a potential write until a later
    /// phase classifies statements by engine-specific parse facts.
    Execute {
        statement: StatementText,
    },
    Cancel {
        operation_id: OperationId,
    },
    /// Apply an authorized mutation plan by review-token handle (never plan bytes).
    ApplyMutations {
        review_token_id: crate::ReviewTokenId,
    },
    Shutdown,
}

impl CommandIntent {
    #[must_use]
    pub const fn safety(&self) -> CommandSafety {
        match self {
            Self::TestProfile | Self::RefreshCatalog | Self::FetchPage(_) => {
                CommandSafety::ReadOnly
            }
            Self::Execute { .. } | Self::ApplyMutations { .. } => CommandSafety::MayWrite,
            Self::Connect | Self::Disconnect | Self::Cancel { .. } | Self::Shutdown => {
                CommandSafety::Lifecycle
            }
        }
    }

    #[must_use]
    pub const fn redaction(&self) -> RedactionClass {
        RedactionClass::MetadataOnly
    }

    const fn scope_matches(&self, scope: CommandScope) -> bool {
        matches!(
            (self, scope),
            (Self::Shutdown, CommandScope::Application)
                | (Self::TestProfile | Self::Connect, CommandScope::Profile(_))
                | (Self::Disconnect, CommandScope::Session { .. })
                | (
                    Self::RefreshCatalog
                        | Self::FetchPage(_)
                        | Self::Execute { .. }
                        | Self::ApplyMutations { .. }
                        | Self::Cancel { .. },
                    CommandScope::Context(_)
                )
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageRequest {
    result_id: ResultId,
    result_revision: Revision,
    start_row: u64,
    row_count: u32,
}

impl PageRequest {
    pub fn new(
        result_id: ResultId,
        result_revision: Revision,
        start_row: u64,
        row_count: u32,
    ) -> Result<Self, CommandBuildError> {
        if row_count == 0 {
            return Err(CommandBuildError::ZeroPageRows);
        }
        if start_row.checked_add(row_count as u64).is_none() {
            return Err(CommandBuildError::PageRangeOverflow);
        }
        Ok(Self {
            result_id,
            result_revision,
            start_row,
            row_count,
        })
    }

    #[must_use]
    pub const fn result_id(self) -> ResultId {
        self.result_id
    }

    #[must_use]
    pub const fn result_revision(self) -> Revision {
        self.result_revision
    }

    #[must_use]
    pub const fn start_row(self) -> u64 {
        self.start_row
    }

    #[must_use]
    pub const fn row_count(self) -> u32 {
        self.row_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandSafety {
    ReadOnly,
    Lifecycle,
    /// Unknown or operator-authored statements; treated as writes until a
    /// later phase proves read-only classification.
    MayWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedactionClass {
    MetadataOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandEnvelope {
    schema_version: u16,
    request_id: RequestId,
    scope: CommandScope,
    expected_revision: Revision,
    budget: ValidatedCommandBudget,
    parent_operation_id: Option<OperationId>,
    intent: CommandIntent,
}

impl CommandEnvelope {
    pub const SCHEMA_VERSION: u16 = 1;

    pub fn new(
        request_id: RequestId,
        scope: CommandScope,
        expected_revision: Revision,
        budget: ValidatedCommandBudget,
        parent_operation_id: Option<OperationId>,
        intent: CommandIntent,
    ) -> Result<Self, CommandBuildError> {
        Self::from_wire(
            Self::SCHEMA_VERSION,
            request_id,
            scope,
            expected_revision,
            budget,
            parent_operation_id,
            intent,
        )
    }

    pub fn from_wire(
        schema_version: u16,
        request_id: RequestId,
        scope: CommandScope,
        expected_revision: Revision,
        budget: ValidatedCommandBudget,
        parent_operation_id: Option<OperationId>,
        intent: CommandIntent,
    ) -> Result<Self, CommandBuildError> {
        if schema_version != Self::SCHEMA_VERSION {
            return Err(CommandBuildError::UnsupportedSchemaVersion {
                actual: schema_version,
                supported: Self::SCHEMA_VERSION,
            });
        }
        if !intent.scope_matches(scope) {
            return Err(CommandBuildError::ScopeMismatch);
        }
        if let CommandIntent::FetchPage(request) = &intent
            && request.row_count() > budget.max_page_rows()
        {
            return Err(CommandBuildError::PageRowsExceedBudget {
                requested: request.row_count(),
                limit: budget.max_page_rows(),
            });
        }
        Ok(Self {
            schema_version,
            request_id,
            scope,
            expected_revision,
            budget,
            parent_operation_id,
            intent,
        })
    }

    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    #[must_use]
    pub const fn request_id(&self) -> RequestId {
        self.request_id
    }

    #[must_use]
    pub const fn scope(&self) -> CommandScope {
        self.scope
    }

    #[must_use]
    pub const fn expected_revision(&self) -> Revision {
        self.expected_revision
    }

    #[must_use]
    pub const fn budget(&self) -> ValidatedCommandBudget {
        self.budget
    }

    #[must_use]
    pub const fn parent_operation_id(&self) -> Option<OperationId> {
        self.parent_operation_id
    }

    #[must_use]
    pub fn intent(&self) -> &CommandIntent {
        &self.intent
    }

    #[must_use]
    pub const fn safety(&self) -> CommandSafety {
        self.intent.safety()
    }

    #[must_use]
    pub const fn redaction(&self) -> RedactionClass {
        self.intent.redaction()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBuildError {
    UnsupportedSchemaVersion { actual: u16, supported: u16 },
    ScopeMismatch,
    ZeroPageRows,
    PageRowsExceedBudget { requested: u32, limit: u32 },
    PageRangeOverflow,
}

impl fmt::Display for CommandBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedSchemaVersion { .. } => "unsupported command schema version",
            Self::ScopeMismatch => "command intent does not belong to the supplied scope",
            Self::ZeroPageRows => "page request row count must be nonzero",
            Self::PageRowsExceedBudget { .. } => "page request exceeds its command row budget",
            Self::PageRangeOverflow => "page request row range overflows",
        })
    }
}

impl Error for CommandBuildError {}
