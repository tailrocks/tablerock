//! Owned, bounded contracts shared by TableRock engines and clients.

mod command;
mod diagnostic;
mod id;
mod operation;
mod page;
mod profile;
mod profile_aggregate;
mod profile_list;
mod revision;
mod secret;
mod value;

pub use command::{
    BudgetField, CommandBudget, CommandBudgetError, CommandBudgetLimits, CommandBuildError,
    CommandEnvelope, CommandIntent, CommandSafety, CommandScope, PageRequest, RedactionClass,
    ValidatedCommandBudget,
};
pub use diagnostic::{
    ApplicationCode, DiagnosticBuildError, DiagnosticPosition, FailureClass, OperationSafety,
    OperatorAction, OutcomeCertainty, PositionUnit, PostgreSqlCode, RedisCode, RetryAdvice,
    SafeCode, SafeDiagnostic, Severity,
};
pub use id::{
    ContextId, IdDecodeError, IdParts, MutationId, OperationId, ProfileId, QueryId, RequestId,
    ResultId, RowId, SessionId, TabId,
};
pub use operation::{
    EventRejection, OperationCursor, OperationEvent, OperationEventKind, OperationIdentity,
    OperationOutcome, OperationPhase, OperationScope, TransitionError,
};
pub use page::{
    CellRef, ColumnMetadata, PageAccessError, PageBuffers, PageDelivery, PageEnvelope, PageFacts,
    PageIdentity, PageLimits, PageShape, PageValidationError, PageWarning, PageWarnings,
    ResultPage, RowTotal, ValidatedPageEnvelope,
};
pub use profile::{
    DangerousTlsAcknowledgement, ProfileBuildError, ProfileConnectionSnapshot, ProfileIdentity,
    ProfileLimitField, ProfileLimits, ProfileName, ProfilePolicy, ProfileProperty,
    ProfilePropertyBinding, ProfilePropertyError, ProfilePropertySet, ProfileSafetyMode,
    PropertyValueSource, TlsPolicy,
};
pub use profile_aggregate::{
    PersistableProfile, ProfileAggregate, ProfileAggregateError, ProfileDurability,
    ProfileGroupName, ProfileLabel, ProfileOrganization, ProfilePreferences, ProfileTag,
    ProfileUpdateError, ReconnectPreference,
};
pub use profile_list::{
    ProfileListCursor, ProfileListError, ProfileListFilter, ProfileListItem, ProfileListPage,
    ProfileListRequest, ProfileSearchTerm, ProfileSourceFacts,
};
pub use revision::{CounterOverflow, EventSequence, Revision, RevisionRelation, SequenceRelation};
pub use secret::{
    DangerousPlaintext, EnvironmentReference, KeychainReference, OnePasswordObjectId,
    OnePasswordReference, OnePasswordSegment, PlaintextAcknowledgement, SecretBuildError,
    SecretField, SecretPersistenceRisk, SecretSource, SecretSourceKind,
};
pub use value::{
    Availability, BoundedBytes, BoundedBytesError, BoundedText, BoundedTextError, ByteLimit,
    Capability, CapabilityEngineMismatch, CapabilityFact, CapabilitySnapshot, EmptyEngineType,
    Engine, EngineType, OwnedValue, Truncation, UnsupportedReason, ValueBuildError, ValueKind,
    ValueRef,
};
