use std::{error::Error, fmt};

use crate::{
    ContextId, EventSequence, OperationId, ProfileId, RequestId, Revision, SequenceRelation,
    SessionId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    scope: OperationScope,
}

impl OperationIdentity {
    #[must_use]
    pub const fn new(
        operation_id: OperationId,
        request_id: RequestId,
        scope: OperationScope,
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
    pub const fn scope(self) -> OperationScope {
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
        match event.sequence.relation_to(self.sequence) {
            SequenceRelation::StaleOrDuplicate => return Err(EventRejection::StaleOrDuplicate),
            SequenceRelation::Gap => return Err(EventRejection::SequenceGap),
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
    pub const fn cumulative_rows(self) -> u64 {
        self.cumulative_rows
    }

    #[must_use]
    pub const fn cumulative_bytes(self) -> u64 {
        self.cumulative_bytes
    }
}
