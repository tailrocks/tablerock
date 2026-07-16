use tablerock_core::{
    CancelRequestOutcome, CommandBudget, CommandBudgetLimits, CommandEnvelope, CommandIntent,
    CommandScope, ContextId, EventQueuePush, IdParts, OperationEventKind, OperationId,
    OperationOutcome, OperationPhase, OperationRetireError, OperationScope, ProfileId, RequestId,
    Revision, ServiceCoordinator, ServiceError, ServiceLimits, ServicePhase, SessionId,
    ShutdownMode, ShutdownOutcome,
};

fn opaque<T>(
    low: u64,
    build: impl FnOnce(IdParts) -> Result<T, tablerock_core::IdDecodeError>,
) -> T {
    build(IdParts::new(0, low).unwrap()).unwrap()
}

fn operation(low: u64) -> OperationId {
    opaque(low, OperationId::from_parts)
}

fn request(low: u64) -> RequestId {
    opaque(low, RequestId::from_parts)
}

fn context(seed: u64) -> OperationScope {
    OperationScope::new(
        opaque(seed, ProfileId::from_parts),
        opaque(seed + 1, SessionId::from_parts),
        opaque(seed + 2, ContextId::from_parts),
    )
}

fn budget() -> tablerock_core::ValidatedCommandBudget {
    CommandBudget::new(10_000, 100, 1_000_000, 1_000)
        .unwrap()
        .validate(CommandBudgetLimits::new(10_000, 100, 1_000_000, 1_000).unwrap())
        .unwrap()
}

fn command(
    request_id: RequestId,
    scope: CommandScope,
    parent: Option<OperationId>,
    intent: CommandIntent,
) -> CommandEnvelope {
    command_at(request_id, scope, Revision::INITIAL, parent, intent)
}

fn command_at(
    request_id: RequestId,
    scope: CommandScope,
    expected_revision: Revision,
    parent: Option<OperationId>,
    intent: CommandIntent,
) -> CommandEnvelope {
    CommandEnvelope::new(
        request_id,
        scope,
        expected_revision,
        budget(),
        parent,
        intent,
    )
    .unwrap()
}

#[test]
fn scoped_revisions_are_hierarchical_monotonic_and_authoritative() {
    let scope = context(50);
    let profile = CommandScope::Profile(scope.profile_id());
    let session = CommandScope::Session {
        profile_id: scope.profile_id(),
        session_id: scope.session_id(),
    };
    let context_scope = CommandScope::Context(scope);
    let mut service = ServiceCoordinator::new(ServiceLimits::new(4, 2, 4).unwrap());
    assert_eq!(
        service.register_scope(context_scope, Revision::INITIAL),
        Err(ServiceError::ParentScopeUnavailable)
    );
    service.register_scope(profile, Revision::INITIAL).unwrap();
    assert_eq!(
        service.register_scope(profile, Revision::INITIAL),
        Err(ServiceError::DuplicateScope)
    );
    service.register_scope(session, Revision::INITIAL).unwrap();
    service
        .register_scope(context_scope, Revision::INITIAL)
        .unwrap();
    assert_eq!(
        service.remove_scope(session),
        Err(ServiceError::ScopeHasChildren)
    );
    let current = service
        .advance_scope(context_scope, Revision::INITIAL)
        .unwrap();
    assert_eq!(current, Revision::from_wire_u64(1));
    assert_eq!(service.scope_revision(context_scope), Some(current));
    assert_eq!(
        service.advance_scope(context_scope, Revision::INITIAL),
        Err(ServiceError::RevisionMismatch {
            expected: Revision::INITIAL,
            current
        })
    );
    assert_eq!(
        service.submit(
            operation(1),
            command(
                request(101),
                context_scope,
                None,
                CommandIntent::RefreshCatalog,
            )
        ),
        Err(ServiceError::RevisionMismatch {
            expected: Revision::INITIAL,
            current
        })
    );
    assert_eq!(
        service.submit(
            operation(1),
            command_at(
                request(101),
                context_scope,
                Revision::from_wire_u64(2),
                None,
                CommandIntent::RefreshCatalog,
            )
        ),
        Err(ServiceError::RevisionMismatch {
            expected: Revision::from_wire_u64(2),
            current
        })
    );
    service
        .submit(
            operation(1),
            command_at(
                request(101),
                context_scope,
                current,
                None,
                CommandIntent::RefreshCatalog,
            ),
        )
        .unwrap();
    let advanced = service.advance_scope(context_scope, current).unwrap();
    assert_eq!(advanced, Revision::from_wire_u64(2));
    assert_eq!(
        service.remove_scope(context_scope),
        Err(ServiceError::ScopeInUse)
    );
    service
        .transition(operation(1), OperationPhase::Running)
        .unwrap();
    assert_eq!(
        service.progress(operation(1), 1, 1),
        Err(ServiceError::RevisionMismatch {
            expected: current,
            current: advanced,
        })
    );
    service
        .transition(
            operation(1),
            OperationPhase::Terminal(OperationOutcome::Completed),
        )
        .unwrap();
    while service.pop_event(operation(1)).unwrap().is_some() {}
    service.retire(operation(1)).unwrap();
    service.remove_scope(context_scope).unwrap();
    service.remove_scope(session).unwrap();
    service.remove_scope(profile).unwrap();
    assert_eq!(
        service.remove_scope(CommandScope::Application),
        Err(ServiceError::ApplicationScopeRequired)
    );
}

#[test]
fn scope_registry_capacity_includes_required_application_scope() {
    let mut service = ServiceCoordinator::new(ServiceLimits::new(1, 1, 1).unwrap());
    assert_eq!(
        service.register_scope(
            CommandScope::Profile(opaque(90, ProfileId::from_parts)),
            Revision::INITIAL
        ),
        Err(ServiceError::ScopeCapacityExceeded)
    );
}

fn context_command(request_seed: u64, parent: Option<OperationId>) -> CommandEnvelope {
    command(
        request(request_seed),
        CommandScope::Context(context(10)),
        parent,
        CommandIntent::RefreshCatalog,
    )
}

fn service(max_operations: u32, event_queue_capacity: u32) -> ServiceCoordinator {
    let scope = context(10);
    let mut service = ServiceCoordinator::new(
        ServiceLimits::new(8, max_operations, event_queue_capacity).unwrap(),
    );
    service
        .register_scope(CommandScope::Profile(scope.profile_id()), Revision::INITIAL)
        .unwrap();
    service
        .register_scope(
            CommandScope::Session {
                profile_id: scope.profile_id(),
                session_id: scope.session_id(),
            },
            Revision::INITIAL,
        )
        .unwrap();
    service
        .register_scope(CommandScope::Context(scope), Revision::INITIAL)
        .unwrap();
    service
}

#[test]
fn service_limits_and_submission_are_finite_and_unique() {
    assert_eq!(
        ServiceLimits::new(0, 1, 1),
        Err(ServiceError::InvalidLimits)
    );
    assert_eq!(
        ServiceLimits::new(1, ServiceLimits::MAX_OPERATIONS + 1, 1),
        Err(ServiceError::InvalidLimits)
    );
    let mut service = service(2, 4);
    let first = service
        .submit(operation(1), context_command(101, None))
        .unwrap();
    assert_eq!(first.scope(), CommandScope::Context(context(10)));
    assert_eq!(
        service.submit(operation(1), context_command(102, None)),
        Err(ServiceError::DuplicateOperation)
    );
    assert_eq!(
        service.submit(operation(2), context_command(101, None)),
        Err(ServiceError::DuplicateRequest)
    );
    service
        .submit(operation(2), context_command(102, Some(operation(1))))
        .unwrap();
    assert_eq!(service.len(), 2);
    assert_eq!(
        service.submit(operation(3), context_command(103, None)),
        Err(ServiceError::OperationCapacityExceeded)
    );
}

#[test]
fn parent_must_exist_remain_active_and_contain_child_scope() {
    let mut service = ServiceCoordinator::new(ServiceLimits::new(4, 4, 4).unwrap());
    let profile_one = opaque(30, ProfileId::from_parts);
    let profile_two = opaque(31, ProfileId::from_parts);
    service
        .register_scope(CommandScope::Profile(profile_one), Revision::INITIAL)
        .unwrap();
    service
        .register_scope(CommandScope::Profile(profile_two), Revision::INITIAL)
        .unwrap();
    assert_eq!(
        service.submit(
            operation(2),
            command(
                request(102),
                CommandScope::Profile(profile_one),
                Some(operation(1)),
                CommandIntent::TestProfile,
            )
        ),
        Err(ServiceError::ParentUnavailable)
    );
    service
        .submit(
            operation(1),
            command(
                request(101),
                CommandScope::Profile(profile_one),
                None,
                CommandIntent::Connect,
            ),
        )
        .unwrap();
    assert_eq!(
        service.submit(
            operation(2),
            command(
                request(102),
                CommandScope::Profile(profile_two),
                Some(operation(1)),
                CommandIntent::TestProfile,
            )
        ),
        Err(ServiceError::ParentScopeMismatch)
    );
    service
        .transition(
            operation(1),
            OperationPhase::Terminal(OperationOutcome::Completed),
        )
        .unwrap_err();
    service
        .transition(operation(1), OperationPhase::Running)
        .unwrap();
    service
        .transition(
            operation(1),
            OperationPhase::Terminal(OperationOutcome::Completed),
        )
        .unwrap();
    assert_eq!(
        service.submit(
            operation(3),
            command(
                request(103),
                CommandScope::Profile(profile_one),
                Some(operation(1)),
                CommandIntent::TestProfile,
            )
        ),
        Err(ServiceError::ParentUnavailable)
    );
}

#[test]
fn coordinator_owns_lifecycle_progress_cancel_and_terminal_delivery() {
    let mut service = service(2, 8);
    service
        .submit(operation(1), context_command(101, None))
        .unwrap();
    assert_eq!(
        service.transition(operation(1), OperationPhase::Running),
        Ok(EventQueuePush::Enqueued)
    );
    assert_eq!(
        service.progress(operation(1), 10, 100),
        Ok(EventQueuePush::Enqueued)
    );
    assert_eq!(
        service.progress(operation(1), 20, 200),
        Ok(EventQueuePush::ProgressCoalesced)
    );
    assert_eq!(
        service.request_cancel(operation(1)),
        Ok(CancelRequestOutcome::Requested)
    );
    assert_eq!(
        service.request_cancel(operation(1)),
        Ok(CancelRequestOutcome::AlreadyRequested)
    );
    service
        .transition(
            operation(1),
            OperationPhase::Terminal(OperationOutcome::CompletedBeforeCancel),
        )
        .unwrap();
    assert_eq!(
        service.request_cancel(operation(1)),
        Ok(CancelRequestOutcome::AlreadyTerminal(
            OperationOutcome::CompletedBeforeCancel
        ))
    );
    assert_eq!(
        service.retire(operation(1)),
        Err(OperationRetireError::PendingEvents)
    );
    let mut kinds = Vec::new();
    while let Some(event) = service.pop_event(operation(1)).unwrap() {
        kinds.push(event.kind());
    }
    assert_eq!(kinds.len(), 4);
    assert!(matches!(kinds[1], OperationEventKind::Progress { .. }));
    service.retire(operation(1)).unwrap();
    assert!(service.is_empty());
}

#[test]
fn shutdown_drains_without_inventing_terminal_outcomes() {
    let mut service = service(3, 8);
    service
        .submit(operation(1), context_command(101, None))
        .unwrap();
    service
        .submit(operation(2), context_command(102, None))
        .unwrap();
    service
        .transition(operation(2), OperationPhase::Running)
        .unwrap();
    assert_eq!(
        service.begin_shutdown(ShutdownMode::CancelActive),
        Ok(ShutdownOutcome::Draining {
            active_operations: 2
        })
    );
    assert_eq!(service.phase(), ServicePhase::Draining);
    assert_eq!(
        service.operation_phase(operation(1)),
        Some(OperationPhase::CancelRequested)
    );
    assert_eq!(
        service.submit(operation(3), context_command(103, None)),
        Err(ServiceError::NotAccepting)
    );
    service
        .transition(
            operation(1),
            OperationPhase::Terminal(OperationOutcome::ClientStopped),
        )
        .unwrap();
    assert_eq!(service.phase(), ServicePhase::Draining);
    service
        .transition(
            operation(2),
            OperationPhase::Terminal(OperationOutcome::Unknown),
        )
        .unwrap();
    assert_eq!(service.phase(), ServicePhase::Stopped);
    assert_eq!(
        service.begin_shutdown(ShutdownMode::Graceful),
        Ok(ShutdownOutcome::AlreadyStopped)
    );
}
