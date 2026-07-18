use tablerock_core::{
    BoundedText, ByteLimit, ColumnMetadata, Engine, EngineType, IdParts, OwnedValue,
    PageAccessError, PageBuffers, PageDelivery, PageEnvelope, PageFacts, PageIdentity, PageLimits,
    PageShape, PageValidationError, PageWarning, PageWarnings, ResultId, ResultPage, Revision,
    RowTotal, Truncation, ValueKind,
};

fn result_id() -> ResultId {
    ResultId::from_parts(IdParts::new(0, 1).unwrap()).unwrap()
}

fn identity() -> PageIdentity {
    PageIdentity::new(result_id(), Revision::INITIAL, Engine::PostgreSql)
}

fn facts() -> PageFacts {
    PageFacts::new(PageDelivery::Partial, PageWarnings::none())
}

#[test]
fn page_warnings_preserve_delivery_discontinuity_independently() {
    let warnings = PageWarnings::none()
        .with(PageWarning::ByteLimitReached)
        .with(PageWarning::DeliveryDiscontinuity);
    assert!(warnings.contains(PageWarning::ByteLimitReached));
    assert!(warnings.contains(PageWarning::DeliveryDiscontinuity));
    assert!(!warnings.contains(PageWarning::PartialFailure));
}

fn columns_for(engine: Engine) -> Vec<ColumnMetadata> {
    vec![
        ColumnMetadata::new(
            BoundedText::copy_from_str("id", ByteLimit::new(2)).unwrap(),
            EngineType::new(
                engine,
                BoundedText::copy_from_str("int8", ByteLimit::new(4)).unwrap(),
            )
            .unwrap(),
            false,
        ),
        ColumnMetadata::new(
            BoundedText::copy_from_str("note", ByteLimit::new(4)).unwrap(),
            EngineType::new(
                engine,
                BoundedText::copy_from_str("text", ByteLimit::new(4)).unwrap(),
            )
            .unwrap(),
            true,
        ),
    ]
}

fn page_envelope() -> PageEnvelope {
    PageEnvelope::new(
        identity(),
        PageShape::new(20, 2, 2, RowTotal::Unknown, 18, 14),
        PageFacts::new(
            PageDelivery::Partial,
            PageWarnings::none().with(PageWarning::UnknownValues),
        ),
    )
}

fn page_limits() -> PageLimits {
    PageLimits::new(500, 64, 1024, 1024)
}

fn validated_envelope() -> tablerock_core::ValidatedPageEnvelope {
    page_envelope().validate(page_limits()).unwrap()
}

fn validated_with_arena(arena_byte_len: u64) -> tablerock_core::ValidatedPageEnvelope {
    PageEnvelope::new(
        identity(),
        PageShape::new(20, 2, 2, RowTotal::Unknown, arena_byte_len, 14),
        facts(),
    )
    .validate(page_limits())
    .unwrap()
}

#[test]
fn page_v1_round_trip_preserves_cells_and_envelope() {
    let text = |value: &str| {
        OwnedValue::text(
            BoundedText::copy_from_str(value, ByteLimit::new(16)).unwrap(),
            Truncation::Complete,
        )
        .unwrap()
    };
    let page = ResultPage::from_row_major(
        identity(),
        10,
        RowTotal::Known(42),
        PageFacts::new(
            PageDelivery::Final,
            PageWarnings::none()
                .with(PageWarning::UnknownValues)
                .with(PageWarning::DeliveryDiscontinuity),
        ),
        columns_for(Engine::PostgreSql),
        vec![
            OwnedValue::signed(7),
            text("alpha"),
            OwnedValue::signed(8),
            OwnedValue::null(),
        ],
        page_limits(),
    )
    .unwrap();

    let encoded = page.encode_v1();
    assert!(encoded.starts_with(b"TRP1"));
    let decoded = ResultPage::decode_v1(&encoded, page_limits()).unwrap();
    assert_eq!(decoded.envelope(), page.envelope());
    assert_eq!(decoded.columns().len(), page.columns().len());
    for row in 0..2 {
        for column in 0..2 {
            let original = page.cell(row, column).unwrap();
            let restored = decoded.cell(row, column).unwrap();
            assert_eq!(restored.kind(), original.kind());
            assert_eq!(restored.is_null(), original.is_null());
            assert_eq!(restored.truncation(), original.truncation());
            assert_eq!(restored.bytes(), original.bytes());
        }
    }
}

#[test]
fn page_v1_decode_rejects_oversized_arena_before_allocation() {
    let page = ResultPage::from_row_major(
        identity(),
        0,
        RowTotal::Unknown,
        facts(),
        columns_for(Engine::PostgreSql),
        vec![
            OwnedValue::signed(1),
            OwnedValue::signed(2),
            OwnedValue::signed(3),
            OwnedValue::signed(4),
        ],
        page_limits(),
    )
    .unwrap();
    let mut encoded = page.encode_v1();
    // Arena length field sits after total_rows (1+8) following start of fixed header.
    // Corrupt the declared arena length to exceed limits while keeping short payload.
    // Fixed header layout after magic:
    // version u16, result_id 16, revision u64, engine u8, start_row u64, row_count u32,
    // column_count u32, total_tag u8, total_value u64, arena_byte_len u64, ...
    let arena_field_offset = 4 + 2 + 16 + 8 + 1 + 8 + 4 + 4 + 1 + 8;
    let huge = (page_limits().max_arena_bytes() + 1).to_le_bytes();
    encoded[arena_field_offset..arena_field_offset + 8].copy_from_slice(&huge);
    let err = ResultPage::decode_v1(&encoded, page_limits()).unwrap_err();
    assert!(matches!(
        err,
        PageValidationError::ArenaLimitExceeded { .. }
    ));
}

#[test]
fn page_v1_decode_rejects_bad_magic() {
    let err = ResultPage::decode_v1(b"XXXX", page_limits()).unwrap_err();
    assert!(matches!(err, PageValidationError::InvalidMagic));
}

#[test]
fn row_major_values_become_validated_column_major_page() {
    let text = |value: &str| {
        OwnedValue::text(
            BoundedText::copy_from_str(value, ByteLimit::new(16)).unwrap(),
            Truncation::Complete,
        )
        .unwrap()
    };
    let page = ResultPage::from_row_major(
        identity(),
        10,
        RowTotal::Unknown,
        facts(),
        columns_for(Engine::PostgreSql),
        vec![
            OwnedValue::signed(7),
            text("alpha"),
            OwnedValue::signed(8),
            OwnedValue::null(),
        ],
        page_limits(),
    )
    .unwrap();

    assert_eq!(page.envelope().row_count(), 2);
    assert_eq!(page.envelope().arena_byte_len(), 21);
    assert_eq!(page.envelope().column_text_byte_len(), 14);
    assert_eq!(page.cell(0, 0).unwrap().bytes(), &7_i64.to_be_bytes());
    assert_eq!(page.cell(1, 0).unwrap().bytes(), &8_i64.to_be_bytes());
    assert_eq!(page.cell(0, 1).unwrap().bytes(), b"alpha");
    assert!(page.cell(1, 1).unwrap().is_null());
}

#[test]
fn row_major_builder_rejects_ragged_or_over_budget_input_before_projection() {
    assert_eq!(
        ResultPage::from_row_major(
            identity(),
            0,
            RowTotal::Unknown,
            facts(),
            columns_for(Engine::PostgreSql),
            vec![OwnedValue::signed(1)],
            page_limits(),
        ),
        Err(PageValidationError::CellCountMismatch {
            expected: 2,
            actual: 1,
        })
    );
    assert_eq!(
        ResultPage::from_row_major(
            identity(),
            0,
            RowTotal::Unknown,
            facts(),
            columns_for(Engine::PostgreSql),
            vec![OwnedValue::signed(1), OwnedValue::null()],
            PageLimits::new(500, 64, 7, 1024),
        ),
        Err(PageValidationError::ArenaLimitExceeded {
            actual: 8,
            limit: 7,
        })
    );
}

fn buffers(
    engine: Engine,
    offsets: Vec<u64>,
    nulls: Vec<u8>,
    kinds: Vec<ValueKind>,
    truncations: Vec<Truncation>,
    arena: Vec<u8>,
) -> PageBuffers {
    PageBuffers::new(
        columns_for(engine),
        offsets,
        nulls,
        kinds,
        truncations,
        arena,
    )
}

fn single_cell_page(kind: ValueKind, bytes: Vec<u8>) -> Result<ResultPage, PageValidationError> {
    let arena_byte_len = bytes.len() as u64;
    let envelope = PageEnvelope::new(
        identity(),
        PageShape::new(0, 1, 1, RowTotal::Known(1), arena_byte_len, 5),
        facts(),
    )
    .validate(page_limits())
    .unwrap();
    ResultPage::from_parts(
        envelope,
        PageBuffers::new(
            vec![ColumnMetadata::new(
                BoundedText::copy_from_str("v", ByteLimit::new(1)).unwrap(),
                EngineType::new(
                    Engine::PostgreSql,
                    BoundedText::copy_from_str("kind", ByteLimit::new(4)).unwrap(),
                )
                .unwrap(),
                true,
            )],
            vec![0, arena_byte_len],
            vec![u8::from(kind == ValueKind::Null)],
            vec![kind],
            vec![Truncation::Complete],
            bytes,
        ),
    )
}

fn kinds() -> Vec<ValueKind> {
    vec![
        ValueKind::Signed,
        ValueKind::Signed,
        ValueKind::Text,
        ValueKind::Null,
    ]
}

fn canonical_arena() -> Vec<u8> {
    let mut arena = Vec::with_capacity(18);
    arena.extend_from_slice(&1_i64.to_be_bytes());
    arena.extend_from_slice(&2_i64.to_be_bytes());
    arena.extend_from_slice(b"hi");
    arena
}

#[test]
fn envelope_rejects_hostile_dimensions_before_page_allocation() {
    let limits = PageLimits::new(500, 64, 1024, 1024);
    let valid = PageEnvelope::new(
        identity(),
        PageShape::new(10, 500, 2, RowTotal::Known(510), 1024, 40),
        facts(),
    );
    assert!(valid.validate(limits).is_ok());

    let oversized = PageEnvelope::new(
        identity(),
        PageShape::new(0, 501, 2, RowTotal::Unknown, 0, 0),
        facts(),
    );
    assert_eq!(
        oversized.validate(limits),
        Err(PageValidationError::RowLimitExceeded {
            actual: 501,
            limit: 500
        })
    );

    let excessive_columns = PageEnvelope::new(
        identity(),
        PageShape::new(0, 1, 65, RowTotal::Unknown, 0, 0),
        facts(),
    );
    assert_eq!(
        excessive_columns.validate(limits),
        Err(PageValidationError::ColumnLimitExceeded {
            actual: 65,
            limit: 64
        })
    );

    let excessive_arena = PageEnvelope::new(
        identity(),
        PageShape::new(0, 1, 1, RowTotal::Unknown, 1025, 0),
        facts(),
    );
    assert_eq!(
        excessive_arena.validate(limits),
        Err(PageValidationError::ArenaLimitExceeded {
            actual: 1025,
            limit: 1024
        })
    );

    let excessive_metadata = PageEnvelope::new(
        identity(),
        PageShape::new(0, 1, 1, RowTotal::Unknown, 0, 1025),
        facts(),
    );
    assert_eq!(
        excessive_metadata.validate(limits),
        Err(PageValidationError::ColumnTextLimitExceeded {
            actual: 1025,
            limit: 1024
        })
    );

    let impossible_total = PageEnvelope::new(
        identity(),
        PageShape::new(10, 2, 1, RowTotal::Known(11), 0, 0),
        facts(),
    );
    assert_eq!(
        impossible_total.validate(limits),
        Err(PageValidationError::KnownTotalBeforePageEnd {
            total: 11,
            page_end: 12
        })
    );

    let overflow = PageEnvelope::new(
        identity(),
        PageShape::new(u64::MAX, 1, 1, RowTotal::Unknown, 0, 0),
        facts(),
    );
    assert_eq!(
        overflow.validate(limits),
        Err(PageValidationError::RowRangeOverflow)
    );
}

#[test]
fn envelope_rejects_unknown_versions_and_preserves_safe_delivery_facts() {
    let envelope = page_envelope();
    assert_eq!(envelope.delivery(), PageDelivery::Partial);
    assert_eq!(envelope.total_rows(), RowTotal::Unknown);
    assert!(envelope.warnings().contains(PageWarning::UnknownValues));

    let unsupported = PageEnvelope::from_wire(
        PageEnvelope::ENCODING_VERSION + 1,
        identity(),
        PageShape::new(0, 0, 0, RowTotal::Known(0), 0, 0),
        facts(),
    );
    assert_eq!(
        unsupported.validate(page_limits()),
        Err(PageValidationError::UnsupportedEncodingVersion {
            actual: 2,
            supported: 1
        })
    );
}

#[test]
fn canonical_encoding_matrix_accepts_every_value_kind_and_rejects_malformed_bytes() {
    let accepted = [
        (ValueKind::Null, vec![]),
        (ValueKind::Boolean, vec![1]),
        (ValueKind::Signed, (-1_i64).to_be_bytes().to_vec()),
        (ValueKind::Unsigned, u64::MAX.to_be_bytes().to_vec()),
        (
            ValueKind::Float64,
            f64::NAN.to_bits().to_be_bytes().to_vec(),
        ),
        (ValueKind::Decimal, b"-12.50".to_vec()),
        (ValueKind::Temporal, b"2024-02-29T12:34:56Z".to_vec()),
        (ValueKind::Text, "snowman: \u{2603}".as_bytes().to_vec()),
        (ValueKind::Structured, br#"[1,{"key":true}]"#.to_vec()),
        (ValueKind::Binary, vec![0, 0xff]),
        (ValueKind::Invalid, vec![0xff]),
        (ValueKind::Unknown, vec![0xfe]),
    ];
    for (kind, bytes) in accepted {
        let page = single_cell_page(kind, bytes.clone()).unwrap();
        let cell = page.cell(0, 0).unwrap();
        assert_eq!(cell.kind(), kind);
        assert_eq!(cell.bytes(), bytes);
    }

    let rejected = [
        (
            ValueKind::Boolean,
            vec![2],
            PageValidationError::InvalidBooleanEncoding { cell: 0 },
        ),
        (
            ValueKind::Unsigned,
            vec![0; 7],
            PageValidationError::InvalidFixedWidthEncoding {
                cell: 0,
                expected: 8,
                actual: 7,
            },
        ),
        (
            ValueKind::Float64,
            vec![0; 9],
            PageValidationError::InvalidFixedWidthEncoding {
                cell: 0,
                expected: 8,
                actual: 9,
            },
        ),
        (
            ValueKind::Decimal,
            vec![0xff],
            PageValidationError::InvalidUtf8Encoding { cell: 0 },
        ),
        (
            ValueKind::Temporal,
            vec![0xff],
            PageValidationError::InvalidUtf8Encoding { cell: 0 },
        ),
    ];
    for (kind, bytes, expected) in rejected {
        assert_eq!(single_cell_page(kind, bytes), Err(expected));
    }
}

#[test]
fn immutable_columnar_page_projects_cells_without_copying_or_logging_values() {
    let page = ResultPage::from_parts(
        validated_envelope(),
        buffers(
            Engine::PostgreSql,
            vec![0, 8, 16, 18, 18],
            vec![0b0000_1000],
            kinds(),
            vec![Truncation::Complete; 4],
            canonical_arena(),
        ),
    )
    .unwrap();

    let id = page.cell(0, 0).unwrap();
    assert_eq!(id.kind(), ValueKind::Signed);
    assert_eq!(id.bytes(), &1_i64.to_be_bytes());
    let null = page.cell(1, 1).unwrap();
    assert!(null.is_null());
    assert!(null.bytes().is_empty());
    assert_eq!(page.cell(0, 1).unwrap().bytes(), b"hi");
    assert!(matches!(
        page.cell(2, 0),
        Err(PageAccessError::RowOutsidePage)
    ));
    assert!(matches!(
        page.cell(0, 2),
        Err(PageAccessError::ColumnOutsidePage)
    ));
    assert!(!format!("{page:?}").contains("hi"));
}

#[test]
fn page_rejects_inconsistent_offsets_nulls_truncation_and_metadata() {
    let invalid_offset = ResultPage::from_parts(
        validated_envelope(),
        buffers(
            Engine::PostgreSql,
            vec![0, 8, 16, 19, 18],
            vec![8],
            kinds(),
            vec![Truncation::Complete; 4],
            canonical_arena(),
        ),
    );
    assert_eq!(
        invalid_offset,
        Err(PageValidationError::OffsetOutsideArena {
            cell: 2,
            offset: 19,
            arena_bytes: 18
        })
    );

    let invalid_null = ResultPage::from_parts(
        validated_envelope(),
        buffers(
            Engine::PostgreSql,
            vec![0, 8, 16, 18, 18],
            vec![0],
            kinds(),
            vec![Truncation::Complete; 4],
            canonical_arena(),
        ),
    );
    assert_eq!(
        invalid_null,
        Err(PageValidationError::NullKindMismatch { cell: 3 })
    );

    let invalid_truncation = ResultPage::from_parts(
        validated_envelope(),
        buffers(
            Engine::PostgreSql,
            vec![0, 8, 16, 18, 18],
            vec![8],
            kinds(),
            vec![
                Truncation::Complete,
                Truncation::Complete,
                Truncation::Truncated {
                    original_byte_len: Some(2),
                },
                Truncation::Complete,
            ],
            canonical_arena(),
        ),
    );
    assert_eq!(
        invalid_truncation,
        Err(PageValidationError::InvalidTruncationLength {
            cell: 2,
            stored: 2,
            original: 2
        })
    );

    let wrong_engine = ResultPage::from_parts(
        validated_envelope(),
        buffers(
            Engine::ClickHouse,
            vec![0, 8, 16, 18, 18],
            vec![8],
            kinds(),
            vec![Truncation::Complete; 4],
            canonical_arena(),
        ),
    );
    assert_eq!(
        wrong_engine,
        Err(PageValidationError::ColumnEngineMismatch { column: 0 })
    );

    let invalid_padding = ResultPage::from_parts(
        validated_envelope(),
        buffers(
            Engine::PostgreSql,
            vec![0, 8, 16, 18, 18],
            vec![0b1000_1000],
            kinds(),
            vec![Truncation::Complete; 4],
            canonical_arena(),
        ),
    );
    assert_eq!(
        invalid_padding,
        Err(PageValidationError::NonzeroNullPadding)
    );
}

#[test]
fn page_rejects_nonnullable_nulls_and_noncanonical_value_encodings() {
    let nonnullable_null = ResultPage::from_parts(
        validated_with_arena(10),
        buffers(
            Engine::PostgreSql,
            vec![0, 0, 8, 10, 10],
            vec![0b0000_1001],
            vec![
                ValueKind::Null,
                ValueKind::Signed,
                ValueKind::Text,
                ValueKind::Null,
            ],
            vec![Truncation::Complete; 4],
            [2_i64.to_be_bytes().as_slice(), b"hi"].concat(),
        ),
    );
    assert_eq!(
        nonnullable_null,
        Err(PageValidationError::NullInNonNullableColumn { cell: 0, column: 0 })
    );

    let short_signed = ResultPage::from_parts(
        validated_envelope(),
        buffers(
            Engine::PostgreSql,
            vec![0, 7, 16, 18, 18],
            vec![8],
            kinds(),
            vec![Truncation::Complete; 4],
            canonical_arena(),
        ),
    );
    assert_eq!(
        short_signed,
        Err(PageValidationError::InvalidFixedWidthEncoding {
            cell: 0,
            expected: 8,
            actual: 7,
        })
    );

    let truncated_signed = ResultPage::from_parts(
        validated_envelope(),
        buffers(
            Engine::PostgreSql,
            vec![0, 8, 16, 18, 18],
            vec![8],
            kinds(),
            vec![
                Truncation::Truncated {
                    original_byte_len: Some(9),
                },
                Truncation::Complete,
                Truncation::Complete,
                Truncation::Complete,
            ],
            canonical_arena(),
        ),
    );
    assert_eq!(
        truncated_signed,
        Err(PageValidationError::UnsupportedTruncationKind {
            cell: 0,
            kind: ValueKind::Signed,
        })
    );

    let mut invalid_utf8 = canonical_arena();
    invalid_utf8[16] = 0xff;
    assert_eq!(
        ResultPage::from_parts(
            validated_envelope(),
            buffers(
                Engine::PostgreSql,
                vec![0, 8, 16, 18, 18],
                vec![8],
                kinds(),
                vec![Truncation::Complete; 4],
                invalid_utf8,
            ),
        ),
        Err(PageValidationError::InvalidUtf8Encoding { cell: 2 })
    );
}
