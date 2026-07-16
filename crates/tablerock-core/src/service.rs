use std::{collections::BTreeMap, error::Error, fmt};

use crate::{
    CommandEnvelope, CommandScope, CounterOverflow, EventQueueError, EventQueuePush, EventSequence,
    OperationCursor, OperationEvent, OperationEventKind, OperationEventQueue, OperationId,
    OperationIdentity, OperationOutcome, OperationPhase, Revision, SubscriptionId, TransitionError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceLimits {
    max_scopes: u32,
    max_operations: u32,
    max_subscriptions_per_operation: u32,
    event_queue_capacity: u32,
}

impl ServiceLimits {
    pub const MAX_SCOPES: u32 = 16_384;
    pub const MAX_OPERATIONS: u32 = 4_096;
    pub const MAX_SUBSCRIPTIONS_PER_OPERATION: u32 = 16;

    pub const fn new(
        max_scopes: u32,
        max_operations: u32,
        max_subscriptions_per_operation: u32,
        event_queue_capacity: u32,
    ) -> Result<Self, ServiceError> {
        if max_scopes == 0
            || max_scopes > Self::MAX_SCOPES
            || max_operations == 0
            || max_operations > Self::MAX_OPERATIONS
            || max_subscriptions_per_operation == 0
            || max_subscriptions_per_operation > Self::MAX_SUBSCRIPTIONS_PER_OPERATION
            || event_queue_capacity == 0
            || event_queue_capacity > OperationEventQueue::MAX_CAPACITY
        {
            return Err(ServiceError::InvalidLimits);
        }
        Ok(Self {
            max_scopes,
            max_operations,
            max_subscriptions_per_operation,
            event_queue_capacity,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServicePhase {
    Accepting,
    Draining,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownMode {
    Graceful,
    CancelActive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownOutcome {
    Draining { active_operations: u32 },
    Stopped,
    AlreadyStopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelRequestOutcome {
    Requested,
    AlreadyRequested,
    AlreadyTerminal(OperationOutcome),
    UnknownOperation,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FanoutOutcome {
    pub subscribers: u32,
    pub enqueued: u32,
    pub progress_coalesced: u32,
    pub resync_required: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionStart {
    Current,
    ResyncQueued,
}

pub struct ServiceCoordinator {
    limits: ServiceLimits,
    phase: ServicePhase,
    revisions: BTreeMap<CommandScope, Revision>,
    operations: BTreeMap<OperationId, OperationRecord>,
}

impl ServiceCoordinator {
    pub fn new(limits: ServiceLimits) -> Self {
        let mut revisions = BTreeMap::new();
        revisions.insert(CommandScope::Application, Revision::INITIAL);
        Self {
            limits,
            phase: ServicePhase::Accepting,
            revisions,
            operations: BTreeMap::new(),
        }
    }

    pub fn register_scope(
        &mut self,
        scope: CommandScope,
        revision: Revision,
    ) -> Result<(), ServiceError> {
        if scope == CommandScope::Application || self.revisions.contains_key(&scope) {
            return Err(ServiceError::DuplicateScope);
        }
        if self.revisions.len() >= self.limits.max_scopes as usize {
            return Err(ServiceError::ScopeCapacityExceeded);
        }
        let parent = parent_scope(scope).expect("application scope rejected above");
        if !self.revisions.contains_key(&parent) {
            return Err(ServiceError::ParentScopeUnavailable);
        }
        self.revisions.insert(scope, revision);
        Ok(())
    }

    pub fn advance_scope(
        &mut self,
        scope: CommandScope,
        expected_revision: Revision,
    ) -> Result<Revision, ServiceError> {
        let current = self
            .revisions
            .get_mut(&scope)
            .ok_or(ServiceError::UnknownScope)?;
        if *current != expected_revision {
            return Err(ServiceError::RevisionMismatch {
                expected: expected_revision,
                current: *current,
            });
        }
        let next = current
            .checked_next()
            .map_err(ServiceError::CounterOverflow)?;
        *current = next;
        Ok(next)
    }

    pub fn remove_scope(&mut self, scope: CommandScope) -> Result<(), ServiceError> {
        if scope == CommandScope::Application {
            return Err(ServiceError::ApplicationScopeRequired);
        }
        if !self.revisions.contains_key(&scope) {
            return Err(ServiceError::UnknownScope);
        }
        if self
            .operations
            .values()
            .any(|record| scope_contains(scope, record.command.scope()))
        {
            return Err(ServiceError::ScopeInUse);
        }
        if self
            .revisions
            .keys()
            .copied()
            .any(|candidate| candidate != scope && scope_contains(scope, candidate))
        {
            return Err(ServiceError::ScopeHasChildren);
        }
        self.revisions.remove(&scope);
        Ok(())
    }

    pub fn submit(
        &mut self,
        operation_id: OperationId,
        command: CommandEnvelope,
    ) -> Result<OperationIdentity, ServiceError> {
        if self.phase != ServicePhase::Accepting {
            return Err(ServiceError::NotAccepting);
        }
        let current_revision = self
            .revisions
            .get(&command.scope())
            .copied()
            .ok_or(ServiceError::UnknownScope)?;
        if command.expected_revision() != current_revision {
            return Err(ServiceError::RevisionMismatch {
                expected: command.expected_revision(),
                current: current_revision,
            });
        }
        if self.operations.len() >= self.limits.max_operations as usize {
            return Err(ServiceError::OperationCapacityExceeded);
        }
        if self.operations.contains_key(&operation_id) {
            return Err(ServiceError::DuplicateOperation);
        }
        if self
            .operations
            .values()
            .any(|record| record.command.request_id() == command.request_id())
        {
            return Err(ServiceError::DuplicateRequest);
        }
        if let Some(parent_id) = command.parent_operation_id() {
            let parent = self
                .operations
                .get(&parent_id)
                .ok_or(ServiceError::ParentUnavailable)?;
            if matches!(parent.cursor.phase(), OperationPhase::Terminal(_)) {
                return Err(ServiceError::ParentUnavailable);
            }
            if !scope_contains(parent.command.scope(), command.scope()) {
                return Err(ServiceError::ParentScopeMismatch);
            }
        }
        let identity = OperationIdentity::new(operation_id, command.request_id(), command.scope());
        let cursor = OperationCursor::new(
            identity,
            Revision::INITIAL,
            EventSequence::INITIAL,
            OperationPhase::Queued,
        );
        self.operations.insert(
            operation_id,
            OperationRecord {
                command,
                cursor,
                subscriptions: BTreeMap::new(),
            },
        );
        Ok(identity)
    }

    pub fn transition(
        &mut self,
        operation_id: OperationId,
        next: OperationPhase,
    ) -> Result<FanoutOutcome, ServiceError> {
        let record = self
            .operations
            .get_mut(&operation_id)
            .ok_or(ServiceError::UnknownOperation)?;
        let from = record.cursor.phase();
        let revision = record
            .cursor
            .revision()
            .checked_next()
            .map_err(ServiceError::CounterOverflow)?;
        let sequence = record
            .cursor
            .sequence()
            .checked_next()
            .map_err(ServiceError::CounterOverflow)?;
        let event = OperationEvent::new(
            record.cursor.identity(),
            revision,
            sequence,
            OperationEventKind::PhaseChanged { from, to: next },
        )
        .map_err(ServiceError::Transition)?;
        record.cursor = record
            .cursor
            .accept(event)
            .expect("coordinator-generated transition must match its cursor");
        let pushed = fanout(record, event);
        self.stop_if_drained();
        Ok(pushed)
    }

    pub fn progress(
        &mut self,
        operation_id: OperationId,
        cumulative_rows: u64,
        cumulative_bytes: u64,
    ) -> Result<FanoutOutcome, ServiceError> {
        let record = self
            .operations
            .get_mut(&operation_id)
            .ok_or(ServiceError::UnknownOperation)?;
        let current_revision = self
            .revisions
            .get(&record.command.scope())
            .copied()
            .ok_or(ServiceError::UnknownScope)?;
        if record.command.expected_revision() != current_revision {
            return Err(ServiceError::RevisionMismatch {
                expected: record.command.expected_revision(),
                current: current_revision,
            });
        }
        let sequence = record
            .cursor
            .sequence()
            .checked_next()
            .map_err(ServiceError::CounterOverflow)?;
        let event = OperationEvent::new(
            record.cursor.identity(),
            record.cursor.revision(),
            sequence,
            OperationEventKind::Progress {
                cumulative_rows,
                cumulative_bytes,
                coalesced_after: None,
            },
        )
        .map_err(ServiceError::Transition)?;
        record.cursor = record
            .cursor
            .accept(event)
            .map_err(ServiceError::EventRejected)?;
        Ok(fanout(record, event))
    }

    pub fn request_cancel(
        &mut self,
        operation_id: OperationId,
    ) -> Result<CancelRequestOutcome, ServiceError> {
        let Some(record) = self.operations.get(&operation_id) else {
            return Ok(CancelRequestOutcome::UnknownOperation);
        };
        match record.cursor.phase() {
            OperationPhase::CancelRequested => Ok(CancelRequestOutcome::AlreadyRequested),
            OperationPhase::Terminal(outcome) => Ok(CancelRequestOutcome::AlreadyTerminal(outcome)),
            OperationPhase::Queued | OperationPhase::Running | OperationPhase::Streaming => {
                self.transition(operation_id, OperationPhase::CancelRequested)?;
                Ok(CancelRequestOutcome::Requested)
            }
        }
    }

    pub fn subscribe(
        &mut self,
        operation_id: OperationId,
        subscription_id: SubscriptionId,
        last_delivered: EventSequence,
    ) -> Result<SubscriptionStart, ServiceError> {
        if self
            .operations
            .values()
            .any(|record| record.subscriptions.contains_key(&subscription_id))
        {
            return Err(ServiceError::DuplicateSubscription);
        }
        let record = self
            .operations
            .get_mut(&operation_id)
            .ok_or(ServiceError::UnknownOperation)?;
        if record.subscriptions.len() >= self.limits.max_subscriptions_per_operation as usize {
            return Err(ServiceError::SubscriptionCapacityExceeded);
        }
        if last_delivered > record.cursor.sequence() {
            return Err(ServiceError::FutureSubscriptionCursor);
        }
        let mut queue = OperationEventQueue::new(
            record.cursor.identity(),
            last_delivered,
            self.limits.event_queue_capacity,
        )
        .map_err(ServiceError::EventQueue)?;
        let start = if last_delivered < record.cursor.sequence() {
            let event = OperationEvent::new(
                record.cursor.identity(),
                record.cursor.revision(),
                record.cursor.sequence(),
                OperationEventKind::ResyncRequired { last_delivered },
            )
            .map_err(ServiceError::Transition)?;
            queue.push(event).map_err(ServiceError::EventQueue)?;
            SubscriptionStart::ResyncQueued
        } else {
            SubscriptionStart::Current
        };
        record.subscriptions.insert(subscription_id, queue);
        Ok(start)
    }

    pub fn unsubscribe(
        &mut self,
        operation_id: OperationId,
        subscription_id: SubscriptionId,
    ) -> Result<(), ServiceError> {
        let record = self
            .operations
            .get_mut(&operation_id)
            .ok_or(ServiceError::UnknownOperation)?;
        record
            .subscriptions
            .remove(&subscription_id)
            .map(|_| ())
            .ok_or(ServiceError::UnknownSubscription)
    }

    pub fn pop_event(
        &mut self,
        operation_id: OperationId,
        subscription_id: SubscriptionId,
    ) -> Result<Option<OperationEvent>, ServiceError> {
        self.operations
            .get_mut(&operation_id)
            .ok_or(ServiceError::UnknownOperation)?
            .subscriptions
            .get_mut(&subscription_id)
            .map(OperationEventQueue::pop_front)
            .ok_or(ServiceError::UnknownSubscription)
    }

    pub fn retire(&mut self, operation_id: OperationId) -> Result<(), OperationRetireError> {
        let record = self
            .operations
            .get(&operation_id)
            .ok_or(OperationRetireError::UnknownOperation)?;
        if !matches!(record.cursor.phase(), OperationPhase::Terminal(_)) {
            return Err(OperationRetireError::StillActive);
        }
        if record.subscriptions.values().any(|queue| !queue.is_empty()) {
            return Err(OperationRetireError::PendingEvents);
        }
        if !record.subscriptions.is_empty() {
            return Err(OperationRetireError::ActiveSubscriptions);
        }
        self.operations.remove(&operation_id);
        Ok(())
    }

    pub fn begin_shutdown(&mut self, mode: ShutdownMode) -> Result<ShutdownOutcome, ServiceError> {
        if self.phase == ServicePhase::Stopped {
            return Ok(ShutdownOutcome::AlreadyStopped);
        }
        self.phase = ServicePhase::Draining;
        if mode == ShutdownMode::CancelActive {
            let active: Vec<_> = self
                .operations
                .iter()
                .filter_map(|(id, record)| {
                    (!matches!(record.cursor.phase(), OperationPhase::Terminal(_))).then_some(*id)
                })
                .collect();
            for operation_id in active {
                self.request_cancel(operation_id)?;
            }
        }
        self.stop_if_drained();
        if self.phase == ServicePhase::Stopped {
            Ok(ShutdownOutcome::Stopped)
        } else {
            Ok(ShutdownOutcome::Draining {
                active_operations: self.active_operations(),
            })
        }
    }

    fn stop_if_drained(&mut self) {
        if self.phase == ServicePhase::Draining && self.active_operations() == 0 {
            self.phase = ServicePhase::Stopped;
        }
    }

    #[must_use]
    pub fn active_operations(&self) -> u32 {
        self.operations
            .values()
            .filter(|record| !matches!(record.cursor.phase(), OperationPhase::Terminal(_)))
            .count() as u32
    }

    #[must_use]
    pub fn operation_phase(&self, operation_id: OperationId) -> Option<OperationPhase> {
        self.operations
            .get(&operation_id)
            .map(|record| record.cursor.phase())
    }

    #[must_use]
    pub fn scope_revision(&self, scope: CommandScope) -> Option<Revision> {
        self.revisions.get(&scope).copied()
    }

    #[must_use]
    pub const fn phase(&self) -> ServicePhase {
        self.phase
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

impl fmt::Debug for ServiceCoordinator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ServiceCoordinator")
            .field("phase", &self.phase)
            .field("scopes", &self.revisions.len())
            .field("operations", &self.operations.len())
            .field("active_operations", &self.active_operations())
            .finish()
    }
}

struct OperationRecord {
    command: CommandEnvelope,
    cursor: OperationCursor,
    subscriptions: BTreeMap<SubscriptionId, OperationEventQueue>,
}

fn fanout(record: &mut OperationRecord, event: OperationEvent) -> FanoutOutcome {
    let mut outcome = FanoutOutcome {
        subscribers: record.subscriptions.len() as u32,
        ..FanoutOutcome::default()
    };
    for queue in record.subscriptions.values_mut() {
        match queue
            .push(event)
            .expect("coordinator-owned subscription queue must accept generated event")
        {
            EventQueuePush::Enqueued => outcome.enqueued += 1,
            EventQueuePush::ProgressCoalesced => outcome.progress_coalesced += 1,
            EventQueuePush::ResyncRequired => outcome.resync_required += 1,
        }
    }
    outcome
}

fn scope_contains(parent: CommandScope, child: CommandScope) -> bool {
    match (parent, child) {
        (CommandScope::Application, _) => true,
        (CommandScope::Profile(parent), CommandScope::Profile(child)) => parent == child,
        (
            CommandScope::Profile(parent),
            CommandScope::Session {
                profile_id: child, ..
            },
        ) => parent == child,
        (CommandScope::Profile(parent), CommandScope::Context(child)) => {
            parent == child.profile_id()
        }
        (
            CommandScope::Session {
                profile_id: parent_profile,
                session_id: parent_session,
            },
            CommandScope::Session {
                profile_id: child_profile,
                session_id: child_session,
            },
        ) => parent_profile == child_profile && parent_session == child_session,
        (CommandScope::Session { .. }, CommandScope::Context(context)) => {
            parent
                == CommandScope::Session {
                    profile_id: context.profile_id(),
                    session_id: context.session_id(),
                }
        }
        (CommandScope::Context(parent), CommandScope::Context(child)) => parent == child,
        _ => false,
    }
}

fn parent_scope(scope: CommandScope) -> Option<CommandScope> {
    match scope {
        CommandScope::Application => None,
        CommandScope::Profile(_) => Some(CommandScope::Application),
        CommandScope::Session { profile_id, .. } => Some(CommandScope::Profile(profile_id)),
        CommandScope::Context(context) => Some(CommandScope::Session {
            profile_id: context.profile_id(),
            session_id: context.session_id(),
        }),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ServiceError {
    InvalidLimits,
    ScopeCapacityExceeded,
    DuplicateScope,
    UnknownScope,
    ParentScopeUnavailable,
    RevisionMismatch {
        expected: Revision,
        current: Revision,
    },
    ScopeInUse,
    ScopeHasChildren,
    ApplicationScopeRequired,
    NotAccepting,
    OperationCapacityExceeded,
    DuplicateOperation,
    DuplicateRequest,
    SubscriptionCapacityExceeded,
    DuplicateSubscription,
    UnknownSubscription,
    FutureSubscriptionCursor,
    ParentUnavailable,
    ParentScopeMismatch,
    UnknownOperation,
    CounterOverflow(CounterOverflow),
    Transition(TransitionError),
    EventRejected(crate::EventRejection),
    EventQueue(EventQueueError),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLimits => "application service limits are invalid",
            Self::ScopeCapacityExceeded => "application service scope capacity exceeded",
            Self::DuplicateScope => "application service scope already exists",
            Self::UnknownScope => "application service scope is unknown",
            Self::ParentScopeUnavailable => "application service parent scope is unavailable",
            Self::RevisionMismatch { .. } => "command aggregate revision is not current",
            Self::ScopeInUse => "application service scope has resident operations",
            Self::ScopeHasChildren => "application service scope has registered children",
            Self::ApplicationScopeRequired => "application scope cannot be removed",
            Self::NotAccepting => "application service is not accepting commands",
            Self::OperationCapacityExceeded => "application service operation capacity exceeded",
            Self::DuplicateOperation => "operation identifier is already active",
            Self::DuplicateRequest => "request identifier is already active",
            Self::SubscriptionCapacityExceeded => "operation subscription capacity exceeded",
            Self::DuplicateSubscription => "subscription identifier is already active",
            Self::UnknownSubscription => "operation subscription is unknown",
            Self::FutureSubscriptionCursor => "subscription cursor is ahead of the operation",
            Self::ParentUnavailable => "parent operation is unavailable",
            Self::ParentScopeMismatch => "child operation escapes its parent scope",
            Self::UnknownOperation => "operation is unknown",
            Self::CounterOverflow(_) => "operation counter exhausted",
            Self::Transition(_) => "operation transition is invalid",
            Self::EventRejected(_) => "operation event was rejected",
            Self::EventQueue(_) => "operation event queue rejected delivery",
        })
    }
}

impl Error for ServiceError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationRetireError {
    UnknownOperation,
    StillActive,
    PendingEvents,
    ActiveSubscriptions,
}

impl fmt::Display for OperationRetireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnknownOperation => "operation is unknown",
            Self::StillActive => "active operation cannot retire",
            Self::PendingEvents => "operation has pending required delivery",
            Self::ActiveSubscriptions => "operation still has active subscriptions",
        })
    }
}

impl Error for OperationRetireError {}
