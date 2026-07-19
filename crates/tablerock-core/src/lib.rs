//! Owned, bounded contracts shared by TableRock engines and clients.

mod catalog;
mod command;
mod connection_url;
mod copy_projection;
mod ddl;
mod diagnostic;
mod editability;
mod id;
mod mutation;
mod named_params;
mod operation;
mod page;
mod profile;
mod profile_aggregate;
mod profile_list;
mod reconnect;
mod redis_command;
mod result_store;
mod revision;
mod secret;
mod service;
mod sql_analysis;
mod sql_format;
mod startup_action;
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
pub use connection_url::{
    ConnectionUrlDraft, ConnectionUrlError, ConnectionUrlTls, MAX_CONNECTION_URL_BYTES,
    parse_connection_url,
};
pub use copy_projection::{
    CopyCell, CopyFormat, CopyProjectionError, CopyTable, copy_cell_from_page, format_copy_table,
};
pub use ddl::{
    DdlBuildError, DdlKind, DdlPlan, DdlTarget, RelationshipEdge, RelationshipGraph,
    RoleMembershipEdge, RoleMembershipGraph, RolePrivilegeRow,
};
pub use diagnostic::{
    ApplicationCode, DiagnosticBuildError, DiagnosticPosition, FailureClass, OperationSafety,
    OperatorAction, OutcomeCertainty, PositionUnit, PostgreSqlCode, RedisCode, RetryAdvice,
    SafeCode, SafeDiagnostic, Severity,
};
pub use editability::{EditabilityFacts, EditabilityReason, StableIdentity};
pub use id::{
    CatalogNodeId, ContextId, IdDecodeError, IdParts, MutationId, OperationId, ProfileId, QueryId,
    RequestId, ResultId, ReviewTokenId, RowId, SessionId, SubscriptionId, TabId,
};
pub use mutation::{
    AuthorizedMutationPlan, FieldValue, MutationBuildError, MutationChange, MutationExecutionModel,
    MutationPlan, MutationPlanLimits, MutationReviewRegistry, MutationTarget, RedisExpiration,
    ReviewError, ReviewRegistryError, ReviewedMutationPlan,
};
pub use named_params::{
    MAX_NAMED_PARAMS, MAX_PARAM_NAME_BYTES, NamedParamError, NamedParamPlan, bind_named_values,
    parse_param_bindings, rewrite_named_params,
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
    EnvironmentTag, PersistableProfile, ProfileAggregate, ProfileAggregateError, ProfileDurability,
    ProfileGroupName, ProfileLabel, ProfileOrganization, ProfilePreferences, ProfileTag,
    ProfileUpdateError, ReconnectPreference,
};
pub use profile_list::{
    ProfileEndpointPart, ProfileEndpointSummary, ProfileListCursor, ProfileListError,
    ProfileListFilter, ProfileListItem, ProfileListPage, ProfileListRequest, ProfileSearchTerm,
    ProfileSourceFacts,
};
pub use reconnect::{ReconnectDecision, reconnect_decision, reconnect_stops_for_redacted_label};
pub use redis_command::{
    RedisCommandLine, RedisCommandPlan, RedisCommandPlanError, RedisCommandSafety,
    RedisPlannedCommand, classify_command as classify_redis_command,
    complete_prefix as complete_redis_command_prefix,
    parse_command_line as parse_redis_command_line, plan_command_text as plan_redis_command_text,
    tokenize as tokenize_redis_command,
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
pub use sql_analysis::{SqlDialect, StatementSpan, statement_at, statements};
pub use sql_format::format_sql;
pub use startup_action::{
    MAX_STARTUP_ACTIONS, MAX_STARTUP_STATEMENT_BYTES, StartupAction, StartupActionError,
    StartupActionOutcome, StartupActionSet, StartupRunReport, StartupSafetyClass,
};
pub use value::{
    Availability, BoundedBytes, BoundedBytesError, BoundedText, BoundedTextError, ByteLimit,
    Capability, CapabilityEngineMismatch, CapabilityFact, CapabilitySnapshot, EmptyEngineType,
    Engine, EngineType, OwnedValue, RedisTimeToLive, Truncation, UnsupportedReason,
    ValueBuildError, ValueKind, ValueRef,
};
