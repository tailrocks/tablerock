use tablerock_core::{
    CommandScope, ContextId, EventQueueError, EventQueuePush, EventRejection, EventSequence,
    IdParts, OperationCursor, OperationEvent, OperationEventKind, OperationEventQueue, OperationId,
    OperationIdentity, OperationOutcome, OperationPhase, OperationScope, ProfileId, RequestId,
    Revision, SessionId, TransitionError,
};

fn id<T>(constructor: impl FnOnce(IdParts) -> Result<T, tablerock_core::IdDecodeError>) -> T {
    constructor(IdParts::new(0, 1).unwrap()).unwrap()
}

fn identity() -> OperationIdentity {
    OperationIdentity::new(
        id(OperationId::from_parts),
        id(RequestId::from_parts),
        CommandScope::Context(OperationScope::new(
            id(ProfileId::from_parts),
            id(SessionId::from_parts),
            id(ContextId::from_parts),
        )),
    )
}

#[test]
fn lifecycle_accepts_streaming_and_observed_cancellation_truth() {
    assert_eq!(
        OperationPhase::Queued.transition_to(OperationPhase::Running),
        Ok(())
    );
    assert_eq!(
        OperationPhase::Running.transition_to(OperationPhase::Streaming),
        Ok(())
    );
    assert_eq!(
        OperationPhase::Streaming.transition_to(OperationPhase::CancelRequested),
        Ok(())
    );
    assert_eq!(
        OperationPhase::CancelRequested.transition_to(OperationPhase::Terminal(
            OperationOutcome::CompletedBeforeCancel
        )),
        Ok(())
    );

    for outcome in [
        OperationOutcome::ClientStopped,
        OperationOutcome::ServerConfirmedCancelled,
        OperationOutcome::CompletedBeforeCancel,
        OperationOutcome::Unknown,
    ] {
        assert_eq!(
            OperationPhase::CancelRequested.transition_to(OperationPhase::Terminal(outcome)),
            Ok(())
        );
    }
}

#[test]
fn operation_identity_covers_every_typed_command_scope() {
    let profile_id = id(ProfileId::from_parts);
    let session_id = id(SessionId::from_parts);
    let context = OperationScope::new(profile_id, session_id, id(ContextId::from_parts));
    for scope in [
        CommandScope::Application,
        CommandScope::Profile(profile_id),
        CommandScope::Session {
            profile_id,
            session_id,
        },
        CommandScope::Context(context),
    ] {
        let identity = OperationIdentity::new(
            id(OperationId::from_parts),
            id(RequestId::from_parts),
            scope,
        );
        assert_eq!(identity.scope(), scope);
    }
}

#[test]
fn lifecycle_rejects_skips_revival_and_false_cancel_claims() {
    assert_eq!(
        OperationPhase::Queued.transition_to(OperationPhase::Streaming),
        Err(TransitionError::IllegalEdge {
            from: OperationPhase::Queued,
            to: OperationPhase::Streaming,
        })
    );
    assert_eq!(
        OperationPhase::Running.transition_to(OperationPhase::Terminal(
            OperationOutcome::ServerConfirmedCancelled
        )),
        Err(TransitionError::CancellationNotRequested)
    );
    assert_eq!(
        OperationPhase::Terminal(OperationOutcome::Unknown).transition_to(OperationPhase::Running),
        Err(TransitionError::TerminalState)
    );
}

#[test]
fn event_cursor_rejects_inconsistent_history_and_requires_resync() {
    let cursor = OperationCursor::new(
        identity(),
        Revision::from_wire_u64(2),
        EventSequence::from_wire_u64(6),
        OperationPhase::Queued,
    );
    let first = OperationEvent::new(
        identity(),
        Revision::from_wire_u64(3),
        EventSequence::from_wire_u64(7),
        OperationEventKind::PhaseChanged {
            from: OperationPhase::Queued,
            to: OperationPhase::Running,
        },
    )
    .unwrap();
    assert!(first.is_required_delivery());
    let cursor = cursor.accept(first).unwrap();
    assert_eq!(cursor.phase(), OperationPhase::Running);

    let progress = OperationEvent::new(
        identity(),
        Revision::from_wire_u64(3),
        EventSequence::from_wire_u64(8),
        OperationEventKind::Progress {
            cumulative_rows: 25,
            cumulative_bytes: 4096,
            coalesced_after: None,
        },
    )
    .unwrap();
    assert!(!progress.is_required_delivery());
    let cursor = cursor.accept(progress).unwrap();
    assert_eq!(cursor.cumulative_rows(), 25);
    assert_eq!(cursor.cumulative_bytes(), 4096);

    let gap = OperationEvent::new(
        identity(),
        Revision::from_wire_u64(3),
        EventSequence::from_wire_u64(10),
        OperationEventKind::Progress {
            cumulative_rows: 30,
            cumulative_bytes: 8192,
            coalesced_after: Some(EventSequence::from_wire_u64(8)),
        },
    )
    .unwrap();
    let cursor = cursor.accept(gap).unwrap();
    assert_eq!(cursor.cumulative_rows(), 30);
    assert_eq!(cursor.cumulative_bytes(), 8192);

    let unproven_gap = OperationEvent::new(
        identity(),
        Revision::from_wire_u64(3),
        EventSequence::from_wire_u64(12),
        OperationEventKind::Progress {
            cumulative_rows: 31,
            cumulative_bytes: 8200,
            coalesced_after: None,
        },
    )
    .unwrap();
    assert_eq!(
        cursor.accept(unproven_gap),
        Err(EventRejection::SequenceGap)
    );

    let resync = OperationEvent::new(
        identity(),
        Revision::from_wire_u64(3),
        EventSequence::from_wire_u64(12),
        OperationEventKind::ResyncRequired {
            last_delivered: EventSequence::from_wire_u64(10),
        },
    )
    .unwrap();
    assert!(resync.is_required_delivery());
    assert_eq!(cursor.accept(resync), Err(EventRejection::ResyncRequired));

    let foreign_identity = OperationIdentity::new(
        OperationId::from_parts(IdParts::new(0, 2).unwrap()).unwrap(),
        identity().request_id(),
        identity().scope(),
    );
    let foreign = OperationEvent::new(
        foreign_identity,
        Revision::from_wire_u64(3),
        EventSequence::from_wire_u64(9),
        OperationEventKind::Progress {
            cumulative_rows: 1,
            cumulative_bytes: 1,
            coalesced_after: None,
        },
    )
    .unwrap();
    assert_eq!(
        cursor.accept(foreign),
        Err(EventRejection::ForeignOperation)
    );

    let stale_revision = OperationEvent::new(
        identity(),
        Revision::from_wire_u64(2),
        EventSequence::from_wire_u64(11),
        OperationEventKind::Progress {
            cumulative_rows: 1,
            cumulative_bytes: 1,
            coalesced_after: None,
        },
    )
    .unwrap();
    assert_eq!(
        cursor.accept(stale_revision),
        Err(EventRejection::RevisionMismatch)
    );

    let duplicate = OperationEvent::new(
        identity(),
        Revision::from_wire_u64(3),
        EventSequence::from_wire_u64(10),
        OperationEventKind::Progress {
            cumulative_rows: 25,
            cumulative_bytes: 4096,
            coalesced_after: None,
        },
    )
    .unwrap();
    assert_eq!(
        cursor.accept(duplicate),
        Err(EventRejection::StaleOrDuplicate)
    );

    let inconsistent_phase = OperationEvent::new(
        identity(),
        Revision::from_wire_u64(4),
        EventSequence::from_wire_u64(11),
        OperationEventKind::PhaseChanged {
            from: OperationPhase::Streaming,
            to: OperationPhase::CancelRequested,
        },
    )
    .unwrap();
    assert_eq!(
        cursor.accept(inconsistent_phase),
        Err(EventRejection::PhaseMismatch)
    );

    let regressed_progress = OperationEvent::new(
        identity(),
        Revision::from_wire_u64(3),
        EventSequence::from_wire_u64(11),
        OperationEventKind::Progress {
            cumulative_rows: 24,
            cumulative_bytes: 4095,
            coalesced_after: None,
        },
    )
    .unwrap();
    assert_eq!(
        cursor.accept(regressed_progress),
        Err(EventRejection::ProgressRegressed)
    );
}

#[test]
fn event_constructor_rejects_illegal_phase_transition() {
    assert_eq!(
        OperationEvent::new(
            identity(),
            Revision::INITIAL,
            EventSequence::INITIAL,
            OperationEventKind::PhaseChanged {
                from: OperationPhase::Queued,
                to: OperationPhase::Streaming,
            },
        ),
        Err(TransitionError::IllegalEdge {
            from: OperationPhase::Queued,
            to: OperationPhase::Streaming,
        })
    );
}

fn progress(sequence: u64, rows: u64) -> OperationEvent {
    OperationEvent::new(
        identity(),
        Revision::from_wire_u64(3),
        EventSequence::from_wire_u64(sequence),
        OperationEventKind::Progress {
            cumulative_rows: rows,
            cumulative_bytes: rows * 10,
            coalesced_after: None,
        },
    )
    .unwrap()
}

#[test]
fn bounded_queue_coalesces_only_consecutive_progress() {
    let mut queue =
        OperationEventQueue::new(identity(), EventSequence::from_wire_u64(7), 2).unwrap();
    assert_eq!(queue.push(progress(8, 10)), Ok(EventQueuePush::Enqueued));
    assert_eq!(
        queue.push(progress(9, 20)),
        Ok(EventQueuePush::ProgressCoalesced)
    );
    assert_eq!(queue.len(), 1);
    let coalesced = queue.pop_front().unwrap();
    assert_eq!(coalesced.sequence().get(), 9);
    assert_eq!(
        coalesced.kind(),
        OperationEventKind::Progress {
            cumulative_rows: 20,
            cumulative_bytes: 200,
            coalesced_after: Some(EventSequence::from_wire_u64(7)),
        }
    );
    assert!(queue.is_empty());
}

#[test]
fn bounded_queue_turns_overflow_and_sequence_loss_into_resync() {
    let mut queue =
        OperationEventQueue::new(identity(), EventSequence::from_wire_u64(7), 1).unwrap();
    let running = OperationEvent::new(
        identity(),
        Revision::from_wire_u64(3),
        EventSequence::from_wire_u64(8),
        OperationEventKind::PhaseChanged {
            from: OperationPhase::Queued,
            to: OperationPhase::Running,
        },
    )
    .unwrap();
    assert_eq!(queue.push(running), Ok(EventQueuePush::Enqueued));
    assert_eq!(
        queue.push(progress(9, 20)),
        Ok(EventQueuePush::ResyncRequired)
    );
    let resync = queue.pop_front().unwrap();
    assert_eq!(
        resync.kind(),
        OperationEventKind::ResyncRequired {
            last_delivered: EventSequence::from_wire_u64(7)
        }
    );
    assert_eq!(
        queue.push(progress(11, 30)),
        Ok(EventQueuePush::ResyncRequired)
    );
}

#[test]
fn bounded_queue_rejects_invalid_capacity_foreign_and_duplicate_events() {
    assert!(matches!(
        OperationEventQueue::new(identity(), EventSequence::INITIAL, 0),
        Err(EventQueueError::InvalidCapacity)
    ));
    assert!(matches!(
        OperationEventQueue::new(
            identity(),
            EventSequence::INITIAL,
            OperationEventQueue::MAX_CAPACITY + 1
        ),
        Err(EventQueueError::InvalidCapacity)
    ));
    let mut queue =
        OperationEventQueue::new(identity(), EventSequence::from_wire_u64(7), 2).unwrap();
    queue.push(progress(8, 10)).unwrap();
    assert_eq!(
        queue.push(progress(8, 10)),
        Err(EventQueueError::StaleOrDuplicate)
    );
    let foreign_identity = OperationIdentity::new(
        OperationId::from_parts(IdParts::new(0, 2).unwrap()).unwrap(),
        identity().request_id(),
        identity().scope(),
    );
    let foreign = OperationEvent::new(
        foreign_identity,
        Revision::from_wire_u64(3),
        EventSequence::from_wire_u64(9),
        OperationEventKind::Progress {
            cumulative_rows: 20,
            cumulative_bytes: 200,
            coalesced_after: None,
        },
    )
    .unwrap();
    assert_eq!(queue.push(foreign), Err(EventQueueError::ForeignOperation));
}
