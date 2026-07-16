use std::mem::size_of;

use tablerock_core::{
    ContextId, EventSequence, IdDecodeError, IdParts, MutationId, OperationId, ProfileId, QueryId,
    RequestId, ResultId, Revision, RevisionRelation, RowId, SequenceRelation, SessionId, TabId,
};

macro_rules! assert_id_contract {
    ($($name:ident),+ $(,)?) => {
        $(
            let parts = IdParts::new(u64::MAX, 0x2a).expect("nonzero ID parts");
            let id = $name::from_parts(parts).expect("valid ID parts");
            assert_eq!(id.parts(), parts);
            assert_eq!(id.to_bytes(), parts.to_bytes());
            assert_eq!($name::from_bytes(id.to_bytes()), Ok(id));
            let text = id.to_string();
            assert_eq!(text, "ffffffffffffffff000000000000002a");
            assert_eq!(text.parse::<$name>(), Ok(id));
            assert_eq!(text.to_uppercase().parse::<$name>(), Ok(id));
            assert_eq!(size_of::<$name>(), 16);
        )+
    };
}

#[test]
fn every_opaque_id_kind_round_trips_canonical_ffi_and_text_encodings() {
    assert_eq!(size_of::<IdParts>(), 16);
    assert_id_contract!(
        ProfileId,
        SessionId,
        ContextId,
        TabId,
        QueryId,
        ResultId,
        RowId,
        MutationId,
        OperationId,
        RequestId,
    );
}

#[test]
fn hostile_or_ambiguous_identity_encodings_are_rejected() {
    assert_eq!(IdParts::new(0, 0), Err(IdDecodeError::Zero));
    assert_eq!(
        ProfileId::from_parts(IdParts { high: 0, low: 0 }),
        Err(IdDecodeError::Zero)
    );
    assert_eq!(ProfileId::from_bytes([0; 16]), Err(IdDecodeError::Zero));
    assert_eq!(
        "0".repeat(32).parse::<ProfileId>(),
        Err(IdDecodeError::Zero)
    );
    assert_eq!(
        "1".repeat(31).parse::<ProfileId>(),
        Err(IdDecodeError::InvalidLength)
    );
    assert_eq!(
        "0000000000000000000000000000000g".parse::<ProfileId>(),
        Err(IdDecodeError::InvalidHex { index: 31 })
    );
    assert_eq!(
        "0000000000000000000000000000002a"
            .parse::<ProfileId>()
            .expect("profile ID")
            .to_string(),
        "0000000000000000000000000000002a"
    );
}

#[test]
fn canonical_id_bytes_use_known_big_endian_boundary_vectors() {
    let minimum = IdParts::new(0, 1).expect("minimum nonzero ID");
    assert_eq!(
        minimum.to_bytes(),
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
    );
    assert_eq!(IdParts::from_bytes(minimum.to_bytes()), Ok(minimum));

    let maximum = IdParts::new(u64::MAX, u64::MAX).expect("maximum ID");
    assert_eq!(maximum.to_bytes(), [0xff; 16]);
    assert_eq!(IdParts::from_bytes([0xff; 16]), Ok(maximum));

    let vector =
        IdParts::new(0x0102_0304_0506_0708, 0x1112_1314_1516_1718).expect("known endian vector");
    assert_eq!(
        vector.to_bytes(),
        [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16,
            0x17, 0x18,
        ]
    );
}

#[test]
fn revisions_classify_stale_current_and_future_without_wrap() {
    let current = Revision::from_wire_u64(7);
    assert_eq!(
        Revision::from_wire_u64(6).relation_to(current),
        RevisionRelation::Stale
    );
    assert_eq!(current.relation_to(current), RevisionRelation::Current);
    assert_eq!(
        Revision::from_wire_u64(8).relation_to(current),
        RevisionRelation::Future
    );
    assert_eq!(current.checked_next().expect("next revision").get(), 8);
    assert!(Revision::from_wire_u64(u64::MAX).checked_next().is_err());
}

#[test]
fn event_sequences_classify_replay_next_and_gap_without_wrap() {
    let last_seen = EventSequence::from_wire_u64(7);
    assert_eq!(
        EventSequence::from_wire_u64(7).relation_to(last_seen),
        SequenceRelation::StaleOrDuplicate
    );
    assert_eq!(
        EventSequence::from_wire_u64(8).relation_to(last_seen),
        SequenceRelation::Next
    );
    assert_eq!(
        EventSequence::from_wire_u64(9).relation_to(last_seen),
        SequenceRelation::Gap
    );
    assert_eq!(
        EventSequence::INITIAL
            .checked_next()
            .expect("first event sequence")
            .get(),
        1
    );
    assert!(
        EventSequence::from_wire_u64(u64::MAX)
            .checked_next()
            .is_err()
    );
}
