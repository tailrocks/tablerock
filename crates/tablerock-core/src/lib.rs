//! Owned, bounded contracts shared by TableRock engines and clients.

mod id;
mod operation;
mod page;
mod revision;
mod value;

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
pub use revision::{CounterOverflow, EventSequence, Revision, RevisionRelation, SequenceRelation};
pub use value::{
    Availability, BoundedBytes, BoundedBytesError, BoundedText, BoundedTextError, ByteLimit,
    Capability, CapabilityEngineMismatch, CapabilityFact, CapabilitySnapshot, EmptyEngineType,
    Engine, EngineType, OwnedValue, Truncation, UnsupportedReason, ValueBuildError, ValueKind,
    ValueRef,
};
