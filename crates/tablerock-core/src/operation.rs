use std::{collections::VecDeque, error::Error, fmt};

use crate::{
    CommandScope, ContextId, EventSequence, OperationId, ProfileId, RequestId, Revision,
    SequenceRelation, SessionId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationScope {
    profile_id: ProfileId,
    session_id: SessionId,
    context_id: ContextId,
}

impl OperationScope {
    #[must_use]
    pub const fn new(profile_id: ProfileId, session_id: SessionId, context_id: ContextId) -> Self {
        Self {
            profile_id,
            session_id,
            context_id,
        }
    }

    #[must_use]
    pub const fn profile_id(self) -> ProfileId {
        self.profile_id
    }

    #[must_use]
    pub const fn session_id(self) -> SessionId {
        self.session_id
    }

    #[must_use]
    pub const fn context_id(self) -> ContextId {
        self.context_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationIdentity {
    operation_id: OperationId,
    request_id: RequestId,
    scope: CommandScope,
}

impl OperationIdentity {
    #[must_use]
    pub const fn new(
        operation_id: OperationId,
        request_id: RequestId,
        scope: CommandScope,
    ) -> Self {
        Self {
            operation_id,
            request_id,
            scope,
        }
    }

    #[must_use]
    pub const fn operation_id(self) -> OperationId {
        self.operation_id
    }

    #[must_use]
    pub const fn request_id(self) -> RequestId {
        self.request_id
    }

    #[must_use]
    pub const fn scope(self) -> CommandScope {
        self.scope
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationOutcome {
    Completed,
    Failed,
    Disconnected,
    ClientStopped,
    ServerConfirmedCancelled,
    CompletedBeforeCancel,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationPhase {
    Queued,
    Running,
    Streaming,
    CancelRequested,
    Terminal(OperationOutcome),
}

impl OperationPhase {
    pub const fn transition_to(self, next: Self) -> Result<(), TransitionError> {
        if matches!(self, Self::Terminal(_)) {
            return Err(TransitionError::TerminalState);
        }
        if matches!(
            next,
            Self::Terminal(
                OperationOutcome::ClientStopped
                    | OperationOutcome::ServerConfirmedCancelled
                    | OperationOutcome::CompletedBeforeCancel
            )
        ) && !matches!(self, Self::CancelRequested)
        {
            return Err(TransitionError::CancellationNotRequested);
        }
        let legal = match (self, next) {
            (Self::Queued, Self::Running | Self::CancelRequested) => true,
            (Self::Queued, Self::Terminal(outcome)) => matches!(
                outcome,
                OperationOutcome::Failed
                    | OperationOutcome::Disconnected
                    | OperationOutcome::Unknown
            ),
            (Self::Running, Self::Streaming | Self::CancelRequested) => true,
            (Self::Streaming, Self::CancelRequested) => true,
            (Self::Running | Self::Streaming, Self::Terminal(outcome)) => matches!(
                outcome,
                OperationOutcome::Completed
                    | OperationOutcome::Failed
                    | OperationOutcome::Disconnected
                    | OperationOutcome::Unknown
            ),
            (Self::CancelRequested, Self::Terminal(outcome)) => !matches!(
                outcome,
                OperationOutcome::Completed | OperationOutcome::Disconnected
            ),
            _ => false,
        };
        if legal {
            Ok(())
        } else {
            Err(TransitionError::IllegalEdge {
                from: self,
                to: next,
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionError {
    IllegalEdge {
        from: OperationPhase,
        to: OperationPhase,
    },
    CancellationNotRequested,
    TerminalState,
}

impl fmt::Display for TransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::IllegalEdge { .. } => "illegal operation lifecycle transition",
            Self::CancellationNotRequested => {
                "cancel outcome requires an observed cancel-requested state"
            }
            Self::TerminalState => "terminal operation cannot transition",
        })
    }
}

impl Error for TransitionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationEventKind {
    PhaseChanged {
        from: OperationPhase,
        to: OperationPhase,
    },
    Progress {
        cumulative_rows: u64,
        cumulative_bytes: u64,
        coalesced_after: Option<EventSequence>,
    },
    ResyncRequired {
        last_delivered: EventSequence,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationEvent {
    identity: OperationIdentity,
    revision: Revision,
    sequence: EventSequence,
    kind: OperationEventKind,
}

impl OperationEvent {
    pub fn new(
        identity: OperationIdentity,
        revision: Revision,
        sequence: EventSequence,
        kind: OperationEventKind,
    ) -> Result<Self, TransitionError> {
        if let OperationEventKind::PhaseChanged { from, to } = kind {
            from.transition_to(to)?;
        }
        Ok(Self {
            identity,
            revision,
            sequence,
            kind,
        })
    }

    #[must_use]
    pub const fn is_required_delivery(self) -> bool {
        !matches!(self.kind, OperationEventKind::Progress { .. })
    }

    #[must_use]
    pub const fn identity(self) -> OperationIdentity {
        self.identity
    }

    #[must_use]
    pub const fn revision(self) -> Revision {
        self.revision
    }

    #[must_use]
    pub const fn sequence(self) -> EventSequence {
        self.sequence
    }

    #[must_use]
    pub const fn kind(self) -> OperationEventKind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventRejection {
    StaleOrDuplicate,
    SequenceGap,
    ForeignOperation,
    RevisionMismatch,
    PhaseMismatch,
    ProgressRegressed,
    ProgressOutsideActivePhase,
    ResyncRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationCursor {
    identity: OperationIdentity,
    revision: Revision,
    sequence: EventSequence,
    phase: OperationPhase,
    cumulative_rows: u64,
    cumulative_bytes: u64,
}

impl OperationCursor {
    #[must_use]
    pub const fn new(
        identity: OperationIdentity,
        revision: Revision,
        sequence: EventSequence,
        phase: OperationPhase,
    ) -> Self {
        Self {
            identity,
            revision,
            sequence,
            phase,
            cumulative_rows: 0,
            cumulative_bytes: 0,
        }
    }

    pub fn accept(self, event: OperationEvent) -> Result<Self, EventRejection> {
        if event.identity != self.identity {
            return Err(EventRejection::ForeignOperation);
        }
        if matches!(event.kind, OperationEventKind::ResyncRequired { .. }) {
            return Err(EventRejection::ResyncRequired);
        }
        match event.sequence.relation_to(self.sequence) {
            SequenceRelation::StaleOrDuplicate => return Err(EventRejection::StaleOrDuplicate),
            SequenceRelation::Gap
                if !matches!(
                    event.kind,
                    OperationEventKind::Progress {
                        coalesced_after: Some(sequence),
                        ..
                    } if sequence == self.sequence
                ) =>
            {
                return Err(EventRejection::SequenceGap);
            }
            SequenceRelation::Gap => {}
            SequenceRelation::Next => {}
        }
        match event.kind {
            OperationEventKind::PhaseChanged { from, to } => {
                if from != self.phase {
                    return Err(EventRejection::PhaseMismatch);
                }
                if self.revision.checked_next().ok() != Some(event.revision) {
                    return Err(EventRejection::RevisionMismatch);
                }
                Ok(Self {
                    revision: event.revision,
                    sequence: event.sequence,
                    phase: to,
                    ..self
                })
            }
            OperationEventKind::Progress {
                cumulative_rows,
                cumulative_bytes,
                ..
            } => {
                if event.revision != self.revision {
                    return Err(EventRejection::RevisionMismatch);
                }
                if !matches!(
                    self.phase,
                    OperationPhase::Running
                        | OperationPhase::Streaming
                        | OperationPhase::CancelRequested
                ) {
                    return Err(EventRejection::ProgressOutsideActivePhase);
                }
                if cumulative_rows < self.cumulative_rows
                    || cumulative_bytes < self.cumulative_bytes
                {
                    return Err(EventRejection::ProgressRegressed);
                }
                Ok(Self {
                    sequence: event.sequence,
                    cumulative_rows,
                    cumulative_bytes,
                    ..self
                })
            }
            OperationEventKind::ResyncRequired { .. } => Err(EventRejection::ResyncRequired),
        }
    }

    #[must_use]
    pub const fn phase(self) -> OperationPhase {
        self.phase
    }

    #[must_use]
    pub const fn identity(self) -> OperationIdentity {
        self.identity
    }

    #[must_use]
    pub const fn revision(self) -> Revision {
        self.revision
    }

    #[must_use]
    pub const fn sequence(self) -> EventSequence {
        self.sequence
    }

    #[must_use]
    pub const fn cumulative_rows(self) -> u64 {
        self.cumulative_rows
    }

    #[must_use]
    pub const fn cumulative_bytes(self) -> u64 {
        self.cumulative_bytes
    }
}

/// Bounded delivery queue for one operation subscription.
///
/// Consecutive cumulative progress events may coalesce. Any other capacity or
/// producer-sequence loss becomes one required resync marker.
pub struct OperationEventQueue {
    identity: OperationIdentity,
    capacity: usize,
    last_delivered: EventSequence,
    events: VecDeque<OperationEvent>,
}

impl OperationEventQueue {
    pub const MAX_CAPACITY: u32 = 4_096;

    pub fn new(
        identity: OperationIdentity,
        last_delivered: EventSequence,
        capacity: u32,
    ) -> Result<Self, EventQueueError> {
        if capacity == 0 || capacity > Self::MAX_CAPACITY {
            return Err(EventQueueError::InvalidCapacity);
        }
        Ok(Self {
            identity,
            capacity: capacity as usize,
            last_delivered,
            events: VecDeque::with_capacity(capacity as usize),
        })
    }

    pub fn push(&mut self, mut event: OperationEvent) -> Result<EventQueuePush, EventQueueError> {
        if event.identity != self.identity {
            return Err(EventQueueError::ForeignOperation);
        }
        let previous = self
            .events
            .back()
            .map_or(self.last_delivered, |queued| queued.sequence);
        match event.sequence.relation_to(previous) {
            SequenceRelation::StaleOrDuplicate => {
                return Err(EventQueueError::StaleOrDuplicate);
            }
            SequenceRelation::Gap => return Ok(self.require_resync(event)),
            SequenceRelation::Next => {}
        }
        if matches!(event.kind, OperationEventKind::Progress { .. })
            && self
                .events
                .back()
                .is_some_and(|queued| matches!(queued.kind, OperationEventKind::Progress { .. }))
        {
            let replaced = self.events.pop_back().expect("progress tail exists");
            let coalesced_after = match replaced.kind {
                OperationEventKind::Progress {
                    coalesced_after: Some(sequence),
                    ..
                } => sequence,
                OperationEventKind::Progress { .. } => self
                    .events
                    .back()
                    .map_or(self.last_delivered, |queued| queued.sequence),
                _ => unreachable!("matched progress tail"),
            };
            if let OperationEventKind::Progress {
                cumulative_rows,
                cumulative_bytes,
                ..
            } = event.kind
            {
                event.kind = OperationEventKind::Progress {
                    cumulative_rows,
                    cumulative_bytes,
                    coalesced_after: Some(coalesced_after),
                };
            }
            self.events.push_back(event);
            return Ok(EventQueuePush::ProgressCoalesced);
        }
        if self.events.len() == self.capacity {
            return Ok(self.require_resync(event));
        }
        self.events.push_back(event);
        Ok(EventQueuePush::Enqueued)
    }

    fn require_resync(&mut self, event: OperationEvent) -> EventQueuePush {
        self.events.clear();
        self.events.push_back(OperationEvent {
            identity: event.identity,
            revision: event.revision,
            sequence: event.sequence,
            kind: OperationEventKind::ResyncRequired {
                last_delivered: self.last_delivered,
            },
        });
        EventQueuePush::ResyncRequired
    }

    pub fn pop_front(&mut self) -> Option<OperationEvent> {
        let event = self.events.pop_front()?;
        self.last_delivered = event.sequence;
        Some(event)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl fmt::Debug for OperationEventQueue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperationEventQueue")
            .field("identity", &self.identity)
            .field("capacity", &self.capacity)
            .field("last_delivered", &self.last_delivered)
            .field("queued", &self.events.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventQueuePush {
    Enqueued,
    ProgressCoalesced,
    ResyncRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventQueueError {
    InvalidCapacity,
    ForeignOperation,
    StaleOrDuplicate,
}

impl fmt::Display for EventQueueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidCapacity => "operation event queue capacity is invalid",
            Self::ForeignOperation => "operation event belongs to another queue",
            Self::StaleOrDuplicate => "operation event sequence is stale or duplicate",
        })
    }
}

impl Error for EventQueueError {}
