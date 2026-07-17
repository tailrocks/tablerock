use tablerock_core::{
    Availability, BoundedBytes, BoundedText, ByteLimit, Capability, CapabilityFact,
    CapabilitySnapshot, EmptyEngineType, Engine, EngineType, OwnedValue, Revision, Truncation,
    UnsupportedReason, ValueBuildError, ValueKind,
};

#[test]
fn capability_facts_keep_engine_differences_explicit() {
    let postgres = CapabilityFact::supported(Engine::PostgreSql, Capability::Transactions);
    assert_eq!(postgres.availability(), Availability::Supported);

    let clickhouse = CapabilityFact::unsupported(
        Engine::ClickHouse,
        Capability::Transactions,
        UnsupportedReason::NotApplicable,
    );
    assert_eq!(clickhouse.engine(), Engine::ClickHouse);
    assert_eq!(clickhouse.capability(), Capability::Transactions);
    assert_eq!(
        clickhouse.availability(),
        Availability::Unsupported(UnsupportedReason::NotApplicable)
    );

    let redis = CapabilityFact::unsupported(
        Engine::Redis,
        Capability::ServerCancellation,
        UnsupportedReason::ProtocolSemantics,
    );
    assert_ne!(redis, clickhouse);

    let snapshot = CapabilitySnapshot::unassessed(Engine::ClickHouse, Revision::INITIAL);
    assert_eq!(snapshot.revision(), Revision::INITIAL);
    for capability in Capability::ALL {
        assert_eq!(snapshot.availability(capability), Availability::Unassessed);
    }
    let snapshot = snapshot.with_fact(clickhouse).unwrap();
    assert_eq!(snapshot.revision(), Revision::INITIAL);
    assert_eq!(
        snapshot.availability(Capability::Transactions),
        Availability::Unsupported(UnsupportedReason::NotApplicable)
    );
    assert!(snapshot.with_fact(postgres).is_err());
}

#[test]
fn byte_and_text_boundaries_reject_before_copy_and_preserve_owned_inputs() {
    let limit = ByteLimit::new(3);
    assert_eq!(
        BoundedBytes::copy_from_slice(b"abc", limit)
            .unwrap()
            .as_slice(),
        b"abc"
    );
    assert_eq!(
        BoundedBytes::copy_from_slice(b"abcd", limit),
        Err(ValueBuildError::ByteLimitExceeded {
            actual: 4,
            limit: 3
        })
    );

    let owned = vec![1, 2, 3, 4];
    let error = BoundedBytes::from_vec(owned, limit).unwrap_err();
    assert_eq!(
        error.kind(),
        ValueBuildError::ByteLimitExceeded {
            actual: 4,
            limit: 3
        }
    );
    assert_eq!(error.into_bytes(), vec![1, 2, 3, 4]);

    let owned = "four".to_owned();
    let error = BoundedText::from_string(owned, limit).unwrap_err();
    assert_eq!(
        error.kind(),
        ValueBuildError::ByteLimitExceeded {
            actual: 4,
            limit: 3
        }
    );
    assert_eq!(error.into_string(), "four");

    assert_eq!(
        BoundedText::copy_from_str("", ByteLimit::new(0))
            .unwrap()
            .as_str(),
        ""
    );
    assert_eq!(
        BoundedText::copy_from_str("é", ByteLimit::new(1)),
        Err(ValueBuildError::ByteLimitExceeded {
            actual: 2,
            limit: 1
        })
    );
}

#[test]
fn owned_values_distinguish_null_empty_binary_float_and_exact_decimal() {
    let empty_text = OwnedValue::text(
        BoundedText::copy_from_str("", ByteLimit::new(0)).unwrap(),
        Truncation::Complete,
    )
    .unwrap();
    let empty_binary = OwnedValue::binary(
        BoundedBytes::copy_from_slice(b"", ByteLimit::new(0)).unwrap(),
        Truncation::Complete,
    )
    .unwrap();

    assert_ne!(OwnedValue::null(), empty_text);
    assert_ne!(empty_text, empty_binary);
    assert_ne!(
        OwnedValue::float64_bits(0.0_f64.to_bits()),
        OwnedValue::float64_bits((-0.0_f64).to_bits())
    );
    assert_eq!(
        OwnedValue::decimal(BoundedText::copy_from_str("1.2300", ByteLimit::new(6)).unwrap()),
        OwnedValue::decimal(BoundedText::copy_from_str("1.2300", ByteLimit::new(6)).unwrap())
    );

    assert_ne!(OwnedValue::boolean(false), OwnedValue::signed(0));
    assert_ne!(OwnedValue::signed(0), OwnedValue::unsigned(0));
    let whitespace = OwnedValue::text(
        BoundedText::copy_from_str(" ", ByteLimit::new(1)).unwrap(),
        Truncation::Complete,
    )
    .unwrap();
    assert_ne!(
        whitespace,
        OwnedValue::text(
            BoundedText::copy_from_str("", ByteLimit::new(0)).unwrap(),
            Truncation::Complete
        )
        .unwrap()
    );
}

#[test]
fn truncation_is_validated_and_unknown_values_retain_engine_type_facts() {
    let prefix = BoundedBytes::copy_from_slice(b"abc", ByteLimit::new(3)).unwrap();
    assert_eq!(
        OwnedValue::binary(
            prefix.clone(),
            Truncation::Truncated {
                original_byte_len: Some(3)
            }
        ),
        Err(ValueBuildError::InvalidTruncationLength {
            stored: 3,
            original: 3
        })
    );

    let engine_type = EngineType::new(
        Engine::ClickHouse,
        BoundedText::copy_from_str("Variant(String, UInt64)", ByteLimit::new(23)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        EngineType::new(
            Engine::ClickHouse,
            BoundedText::copy_from_str("", ByteLimit::new(0)).unwrap()
        ),
        Err(EmptyEngineType)
    );
    let unknown = OwnedValue::unknown(
        engine_type.clone(),
        prefix,
        Truncation::Truncated {
            original_byte_len: None,
        },
    )
    .unwrap();
    assert_eq!(unknown.engine_type(), Some(&engine_type));
    assert!(unknown.is_truncated());

    let invalid = OwnedValue::invalid(
        engine_type,
        BoundedBytes::copy_from_slice(b"bad", ByteLimit::new(3)).unwrap(),
        Truncation::Complete,
    )
    .unwrap();
    assert_eq!(invalid.kind(), ValueKind::Invalid);
    assert_ne!(invalid, unknown);
}

#[test]
fn value_debug_output_never_contains_cell_or_type_content() {
    let secret = BoundedText::copy_from_str("do-not-log", ByteLimit::new(10)).unwrap();
    let value = OwnedValue::text(secret, Truncation::Complete).unwrap();
    let debug = format!("{value:?}");
    assert!(!debug.contains("do-not-log"));
    assert!(debug.contains("Text"));

    let structured = OwnedValue::structured(
        BoundedText::copy_from_str("[\"do-not-log\"]", ByteLimit::new(14)).unwrap(),
        Truncation::Complete,
    )
    .unwrap();
    assert_eq!(structured.kind(), ValueKind::Structured);

    let temporal = OwnedValue::temporal(
        BoundedText::copy_from_str("2024-02-29T12:34:56Z", ByteLimit::new(20)).unwrap(),
        Truncation::Complete,
    )
    .unwrap();
    assert_eq!(temporal.kind(), ValueKind::Temporal);
    assert!(matches!(
        temporal.as_ref(),
        tablerock_core::ValueRef::Temporal {
            value: "2024-02-29T12:34:56Z",
            truncation: Truncation::Complete
        }
    ));
    assert!(!format!("{structured:?}").contains("do-not-log"));

    let engine_type = EngineType::new(
        Engine::PostgreSql,
        BoundedText::copy_from_str("secret_type", ByteLimit::new(11)).unwrap(),
    )
    .unwrap();
    assert!(!format!("{engine_type:?}").contains("secret_type"));

    let payload = BoundedBytes::copy_from_slice(b"payload-secret", ByteLimit::new(14)).unwrap();
    let unknown = OwnedValue::unknown(engine_type, payload, Truncation::Complete).unwrap();
    assert!(!format!("{unknown:?}").contains("payload-secret"));
}
