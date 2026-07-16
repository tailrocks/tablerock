use tablerock_core::{
    ContextId, EventRejection, EventSequence, IdParts, OperationCursor, OperationEvent,
    OperationEventKind, OperationId, OperationIdentity, OperationOutcome, OperationPhase,
    OperationScope, ProfileId, RequestId, Revision, SessionId, TransitionError,
};

fn id<T>(constructor: impl FnOnce(IdParts) -> Result<T, tablerock_core::IdDecodeError>) -> T {
    constructor(IdParts::new(0, 1).unwrap()).unwrap()
}

fn identity() -> OperationIdentity {
    OperationIdentity::new(
        id(OperationId::from_parts),
        id(RequestId::from_parts),
        OperationScope::new(
            id(ProfileId::from_parts),
            id(SessionId::from_parts),
            id(ContextId::from_parts),
        ),
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
        },
    )
    .unwrap();
    assert_eq!(cursor.accept(gap), Err(EventRejection::SequenceGap));

    let resync = OperationEvent::new(
        identity(),
        Revision::from_wire_u64(3),
        EventSequence::from_wire_u64(9),
        OperationEventKind::ResyncRequired {
            last_delivered: EventSequence::from_wire_u64(8),
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
        EventSequence::from_wire_u64(9),
        OperationEventKind::Progress {
            cumulative_rows: 1,
            cumulative_bytes: 1,
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
        EventSequence::from_wire_u64(8),
        OperationEventKind::Progress {
            cumulative_rows: 25,
            cumulative_bytes: 4096,
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
        EventSequence::from_wire_u64(9),
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
        EventSequence::from_wire_u64(9),
        OperationEventKind::Progress {
            cumulative_rows: 24,
            cumulative_bytes: 4095,
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
