//! Owned, bounded contracts shared by TableRock engines and clients.

mod catalog;
mod command;
mod diagnostic;
mod id;
mod mutation;
mod operation;
mod page;
mod profile;
mod profile_aggregate;
mod profile_list;
mod result_store;
mod revision;
mod secret;
mod service;
mod value;

pub use catalog::{
    CatalogBuildError, CatalogChildrenState, CatalogCursor, CatalogIdentity, CatalogLimits,
    CatalogNode, CatalogNodeKind, CatalogRejection, CatalogSnapshot, ClickHouseObjectKind,
    PostgreSqlObjectKind, RedisKeyKind,
};
pub use command::{
    BudgetField, CommandBudget, CommandBudgetError, CommandBudgetLimits, CommandBuildError,
    CommandEnvelope, CommandIntent, CommandSafety, CommandScope, MAX_STATEMENT_BYTES, PageRequest,
    RedactionClass, StatementText, StatementTextError, ValidatedCommandBudget,
};
pub use diagnostic::{
    ApplicationCode, DiagnosticBuildError, DiagnosticPosition, FailureClass, OperationSafety,
    OperatorAction, OutcomeCertainty, PositionUnit, PostgreSqlCode, RedisCode, RetryAdvice,
    SafeCode, SafeDiagnostic, Severity,
};
pub use id::{
    CatalogNodeId, ContextId, IdDecodeError, IdParts, MutationId, OperationId, ProfileId, QueryId,
    RequestId, ResultId, ReviewTokenId, RowId, SessionId, SubscriptionId, TabId,
};
pub use mutation::{
    AuthorizedMutationPlan, FieldValue, MutationBuildError, MutationChange, MutationExecutionModel,
    MutationPlan, MutationPlanLimits, MutationReviewRegistry, MutationTarget, RedisExpiration,
    ReviewError, ReviewRegistryError, ReviewedMutationPlan,
};
pub use operation::{
    CancelDispatch, EventQueueError, EventQueuePush, EventRejection, OperationCursor,
    OperationEvent, OperationEventKind, OperationEventQueue, OperationIdentity, OperationOutcome,
    OperationPhase, OperationScope, TransitionError,
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
    ProfileEndpointPart, ProfileEndpointSummary, ProfileListCursor, ProfileListError,
    ProfileListFilter, ProfileListItem, ProfileListPage, ProfileListRequest, ProfileSearchTerm,
    ProfileSourceFacts,
};
pub use result_store::{
    AdmissionOutcome, OpenResultOutcome, PageKey, ResultStore, ResultStoreError, ResultStoreLimits,
};
pub use revision::{CounterOverflow, EventSequence, Revision, RevisionRelation, SequenceRelation};
pub use secret::{
    DangerousPlaintext, EnvironmentReference, KeychainReference, OnePasswordObjectId,
    OnePasswordReference, OnePasswordSegment, PlaintextAcknowledgement, SecretBuildError,
    SecretField, SecretPersistenceRisk, SecretSource, SecretSourceKind,
};
pub use service::{
    CancelRequestOutcome, FanoutOutcome, OperationRetireError, ServiceCoordinator, ServiceError,
    ServiceLimits, ServicePhase, ShutdownMode, ShutdownOutcome, SubscriptionStart,
};
pub use value::{
    Availability, BoundedBytes, BoundedBytesError, BoundedText, BoundedTextError, ByteLimit,
    Capability, CapabilityEngineMismatch, CapabilityFact, CapabilitySnapshot, EmptyEngineType,
    Engine, EngineType, OwnedValue, RedisTimeToLive, Truncation, UnsupportedReason,
    ValueBuildError, ValueKind, ValueRef,
};
