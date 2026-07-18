use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, ContextId, Engine, EngineType, FieldValue, IdParts,
    MutationBuildError, MutationChange, MutationExecutionModel, MutationId, MutationPlan,
    MutationPlanLimits, MutationReviewRegistry, MutationTarget, OperationScope, OwnedValue,
    ProfileId, RedisExpiration, ReviewError, ReviewRegistryError, ReviewTokenId, Revision,
    SessionId, Truncation,
};

fn opaque<T>(
    low: u64,
    build: impl FnOnce(IdParts) -> Result<T, tablerock_core::IdDecodeError>,
) -> T {
    build(IdParts::new(0, low).unwrap()).unwrap()
}

fn scope(seed: u64) -> OperationScope {
    OperationScope::new(
        opaque(seed, ProfileId::from_parts),
        opaque(seed + 1, SessionId::from_parts),
        opaque(seed + 2, ContextId::from_parts),
    )
}

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(1024)).unwrap()
}

fn bytes(value: &[u8]) -> BoundedBytes {
    BoundedBytes::copy_from_slice(value, ByteLimit::new(1024)).unwrap()
}

fn field(name: &str, value: OwnedValue) -> FieldValue {
    FieldValue::new(text(name), value)
}

fn limits() -> MutationPlanLimits {
    MutationPlanLimits::new(16, 16, 4096, 4096, 10_000).unwrap()
}

fn mutation_id() -> MutationId {
    opaque(20, MutationId::from_parts)
}

fn review_id() -> ReviewTokenId {
    opaque(21, ReviewTokenId::from_parts)
}

fn review_id_with(seed: u64) -> ReviewTokenId {
    opaque(seed, ReviewTokenId::from_parts)
}

fn postgres_target() -> MutationTarget {
    MutationTarget::PostgreSqlRelation {
        database: text("app"),
        schema: text("public"),
        relation: text("users"),
    }
}

fn clickhouse_target() -> MutationTarget {
    MutationTarget::ClickHouseTable {
        database: text("analytics"),
        table: text("events"),
    }
}

fn plan(target: MutationTarget, changes: Vec<MutationChange>) -> MutationPlan {
    MutationPlan::new(
        mutation_id(),
        scope(1),
        Revision::from_wire_u64(7),
        target,
        changes,
        limits(),
    )
    .unwrap()
}

#[test]
fn review_consumes_exact_typed_postgresql_plan() {
    let plan = plan(
        postgres_target(),
        vec![MutationChange::UpdateRow {
            locator: vec![field("id", OwnedValue::unsigned(42))],
            assignments: vec![field(
                "email",
                OwnedValue::text(text("private@example.test"), Truncation::Complete).unwrap(),
            )],
        }],
    );
    assert_eq!(
        plan.execution_model(),
        MutationExecutionModel::PostgreSqlAtomicTransaction
    );
    let reviewed = plan.review(review_id(), 1_000, 2_000).unwrap();
    let authorized = reviewed
        .authorize(1_500, scope(1), Revision::from_wire_u64(7))
        .unwrap();
    assert_eq!(authorized.token_id(), review_id());
    assert_eq!(authorized.plan().changes().len(), 1);
    assert_eq!(authorized.plan().mutation_id(), mutation_id());
}

#[test]
fn clickhouse_insert_and_async_mutation_models_never_mix() {
    let insert = plan(
        clickhouse_target(),
        vec![MutationChange::InsertRow {
            values: vec![field("id", OwnedValue::unsigned(1))],
        }],
    );
    assert_eq!(
        insert.execution_model(),
        MutationExecutionModel::ClickHouseProgressiveInsertNonTransactional
    );
    let deletion = plan(
        clickhouse_target(),
        vec![MutationChange::DeleteRow {
            locator: vec![field("id", OwnedValue::unsigned(1))],
        }],
    );
    assert_eq!(
        deletion.execution_model(),
        MutationExecutionModel::ClickHouseAsynchronousMutationNonTransactional
    );
    assert!(matches!(
        MutationPlan::new(
            mutation_id(),
            scope(1),
            Revision::INITIAL,
            clickhouse_target(),
            vec![
                MutationChange::InsertRow {
                    values: vec![field("id", OwnedValue::unsigned(1))]
                },
                MutationChange::DeleteRow {
                    locator: vec![field("id", OwnedValue::unsigned(1))]
                }
            ],
            limits(),
        ),
        Err(MutationBuildError::MixedClickHouseExecutionModels { change: 1 })
    ));
}

#[test]
fn redis_collection_mutations_are_sequential_and_engine_gated() {
    let collection_plan = plan(
        MutationTarget::RedisKey {
            logical_database: 0,
            key: bytes(b"col"),
        },
        vec![
            MutationChange::RedisHashSetField {
                field: bytes(b"f"),
                value: bytes(b"v"),
            },
            MutationChange::RedisSetAddMember {
                member: bytes(b"m"),
            },
            MutationChange::RedisZSetAddMember {
                member: bytes(b"z"),
                score_bits: 1.5_f64.to_bits(),
            },
        ],
    );
    assert_eq!(
        collection_plan.execution_model(),
        MutationExecutionModel::RedisSequentialNoRollback
    );
    assert_eq!(collection_plan.changes().len(), 3);
    assert!(matches!(
        MutationPlan::new(
            mutation_id(),
            scope(1),
            Revision::INITIAL,
            postgres_target(),
            vec![MutationChange::RedisHashSetField {
                field: bytes(b"f"),
                value: bytes(b"v"),
            }],
            limits(),
        ),
        Err(MutationBuildError::ChangeEngineMismatch { change: 0 })
    ));
    assert!(matches!(
        MutationPlan::new(
            mutation_id(),
            scope(1),
            Revision::INITIAL,
            MutationTarget::RedisKey {
                logical_database: 0,
                key: bytes(b"k"),
            },
            vec![MutationChange::RedisHashDeleteField {
                field: bytes(b""),
            }],
            limits(),
        ),
        Err(MutationBuildError::EmptyFields { change: 0 })
    ));
}

#[test]
fn redis_plan_keeps_raw_key_value_ttl_and_no_rollback_truth() {
    let target = MutationTarget::RedisKey {
        logical_database: 3,
        key: bytes(&[0, 255]),
    };
    let plan = plan(
        target,
        vec![MutationChange::RedisSetString {
            value: bytes(&[1, 2, 0, 255]),
            expiration: RedisExpiration::Preserve,
        }],
    );
    assert_eq!(plan.target().engine(), Engine::Redis);
    assert_eq!(
        plan.execution_model(),
        MutationExecutionModel::RedisSequentialNoRollback
    );
    assert!(matches!(
        MutationPlan::new(
            mutation_id(),
            scope(1),
            Revision::INITIAL,
            MutationTarget::RedisKey {
                logical_database: 0,
                key: bytes(b"key")
            },
            vec![MutationChange::RedisSetExpiration(
                RedisExpiration::ExpireAfterMillis(0)
            )],
            limits(),
        ),
        Err(MutationBuildError::InvalidExpiration { change: 0 })
    ));
    assert!(matches!(
        MutationPlan::new(
            mutation_id(),
            scope(1),
            Revision::INITIAL,
            MutationTarget::RedisKey {
                logical_database: 0,
                key: bytes(b"key")
            },
            vec![MutationChange::RedisSetExpiration(
                RedisExpiration::ExpireAfterMillis(i64::MAX as u64 + 1)
            )],
            limits(),
        ),
        Err(MutationBuildError::InvalidExpiration { change: 0 })
    ));
    assert!(matches!(
        MutationPlan::new(
            mutation_id(),
            scope(1),
            Revision::INITIAL,
            MutationTarget::RedisKey {
                logical_database: 0,
                key: bytes(b"key")
            },
            vec![MutationChange::RedisSetExpiration(
                RedisExpiration::Preserve
            )],
            limits(),
        ),
        Err(MutationBuildError::InvalidExpiration { change: 0 })
    ));
}

#[test]
fn rejects_cross_engine_changes_and_non_executable_values() {
    assert!(matches!(
        MutationPlan::new(
            mutation_id(),
            scope(1),
            Revision::INITIAL,
            postgres_target(),
            vec![MutationChange::RedisDeleteKey],
            limits(),
        ),
        Err(MutationBuildError::ChangeEngineMismatch { change: 0 })
    ));

    let engine_type = EngineType::new(Engine::PostgreSql, text("extension_type")).unwrap();
    let unknown = OwnedValue::unknown(engine_type, bytes(b"opaque"), Truncation::Complete).unwrap();
    assert!(matches!(
        MutationPlan::new(
            mutation_id(),
            scope(1),
            Revision::INITIAL,
            postgres_target(),
            vec![MutationChange::InsertRow {
                values: vec![field("payload", unknown)]
            }],
            limits(),
        ),
        Err(MutationBuildError::NonExecutableValue {
            change: 0,
            field: 0
        })
    ));
    assert!(matches!(
        MutationPlan::new(
            mutation_id(),
            scope(1),
            Revision::INITIAL,
            postgres_target(),
            vec![MutationChange::DeleteRow {
                locator: vec![field("id", OwnedValue::null())]
            }],
            limits(),
        ),
        Err(MutationBuildError::NullLocator {
            change: 0,
            field: 0
        })
    ));
}

#[test]
fn rejects_duplicate_fields_and_every_aggregate_bound() {
    assert!(matches!(
        MutationPlan::new(
            mutation_id(),
            scope(1),
            Revision::INITIAL,
            postgres_target(),
            vec![MutationChange::InsertRow {
                values: vec![
                    field("id", OwnedValue::unsigned(1)),
                    field("id", OwnedValue::unsigned(2))
                ]
            }],
            limits(),
        ),
        Err(MutationBuildError::DuplicateField {
            change: 0,
            field: 1
        })
    ));
    assert_eq!(
        MutationPlanLimits::new(0, 1, 1, 1, 1),
        Err(MutationBuildError::InvalidLimits)
    );
    assert!(matches!(
        MutationPlan::new(
            mutation_id(),
            scope(1),
            Revision::INITIAL,
            postgres_target(),
            vec![MutationChange::InsertRow {
                values: vec![field("id", OwnedValue::unsigned(1))]
            }],
            MutationPlanLimits::new(1, 1, 1, 1, 1).unwrap(),
        ),
        Err(MutationBuildError::TextLimitExceeded { .. })
    ));
}

#[test]
fn review_expiry_scope_and_revision_are_fail_closed() {
    assert!(matches!(
        plan(
            postgres_target(),
            vec![MutationChange::DeleteRow {
                locator: vec![field("id", OwnedValue::unsigned(1))]
            }]
        )
        .review(review_id(), 100, 100),
        Err(ReviewError::InvalidExpiry)
    ));
    let reviewed = plan(
        postgres_target(),
        vec![MutationChange::DeleteRow {
            locator: vec![field("id", OwnedValue::unsigned(1))],
        }],
    )
    .review(review_id(), 100, 200)
    .unwrap();
    assert!(matches!(
        reviewed.authorize(200, scope(1), Revision::from_wire_u64(7)),
        Err(ReviewError::Expired)
    ));
    let reviewed = plan(
        postgres_target(),
        vec![MutationChange::DeleteRow {
            locator: vec![field("id", OwnedValue::unsigned(1))],
        }],
    )
    .review(review_id(), 100, 200)
    .unwrap();
    assert!(matches!(
        reviewed.authorize(150, scope(9), Revision::from_wire_u64(7)),
        Err(ReviewError::ScopeMismatch)
    ));
    let reviewed = plan(
        postgres_target(),
        vec![MutationChange::DeleteRow {
            locator: vec![field("id", OwnedValue::unsigned(1))],
        }],
    )
    .review(review_id(), 100, 200)
    .unwrap();
    assert!(matches!(
        reviewed.authorize(150, scope(1), Revision::from_wire_u64(8)),
        Err(ReviewError::RevisionMismatch)
    ));
}

#[test]
fn debug_never_contains_identifiers_keys_or_values() {
    let plan = plan(
        MutationTarget::RedisKey {
            logical_database: 0,
            key: bytes(b"do-not-log-key"),
        },
        vec![MutationChange::RedisSetString {
            value: bytes(b"do-not-log-value"),
            expiration: RedisExpiration::Persist,
        }],
    );
    let debug = format!("{plan:?}");
    assert!(!debug.contains("do-not-log"));
    assert!(debug.contains("value_bytes"));
}

#[test]
fn registry_redeems_authority_exactly_once() {
    let reviewed = plan(
        postgres_target(),
        vec![MutationChange::DeleteRow {
            locator: vec![field("id", OwnedValue::unsigned(1))],
        }],
    )
    .review(review_id(), 100, 200)
    .unwrap();
    let mut registry = MutationReviewRegistry::new(1).unwrap();
    registry.insert(reviewed, 100).unwrap();
    assert_eq!(registry.len(), 1);
    let authorized = registry
        .authorize(review_id(), 150, scope(1), Revision::from_wire_u64(7))
        .unwrap();
    assert_eq!(authorized.token_id(), review_id());
    assert!(registry.is_empty());
    assert!(matches!(
        registry.authorize(review_id(), 150, scope(1), Revision::from_wire_u64(7)),
        Err(ReviewRegistryError::TokenNotFound)
    ));
}

#[test]
fn failed_registry_authorization_still_consumes_authority() {
    let reviewed = plan(
        postgres_target(),
        vec![MutationChange::DeleteRow {
            locator: vec![field("id", OwnedValue::unsigned(1))],
        }],
    )
    .review(review_id(), 100, 200)
    .unwrap();
    let mut registry = MutationReviewRegistry::new(1).unwrap();
    registry.insert(reviewed, 100).unwrap();
    assert!(matches!(
        registry.authorize(review_id(), 150, scope(9), Revision::from_wire_u64(7)),
        Err(ReviewRegistryError::Review(ReviewError::ScopeMismatch))
    ));
    assert!(registry.is_empty());
}

#[test]
fn registry_bounds_duplicates_expiry_and_revocation() {
    assert!(matches!(
        MutationReviewRegistry::new(0),
        Err(ReviewRegistryError::InvalidCapacity)
    ));
    assert!(matches!(
        MutationReviewRegistry::new(MutationReviewRegistry::MAX_ENTRIES + 1),
        Err(ReviewRegistryError::InvalidCapacity)
    ));
    let reviewed = plan(
        postgres_target(),
        vec![MutationChange::DeleteRow {
            locator: vec![field("id", OwnedValue::unsigned(1))],
        }],
    )
    .review(review_id_with(30), 100, 200)
    .unwrap();
    let duplicate = plan(
        postgres_target(),
        vec![MutationChange::DeleteRow {
            locator: vec![field("id", OwnedValue::unsigned(2))],
        }],
    )
    .review(review_id_with(30), 100, 200)
    .unwrap();
    let overflow = plan(
        postgres_target(),
        vec![MutationChange::DeleteRow {
            locator: vec![field("id", OwnedValue::unsigned(3))],
        }],
    )
    .review(review_id_with(31), 100, 200)
    .unwrap();
    let mut registry = MutationReviewRegistry::new(1).unwrap();
    registry.insert(reviewed, 100).unwrap();
    assert!(matches!(
        registry.insert(duplicate, 100),
        Err(ReviewRegistryError::DuplicateToken)
    ));
    assert!(matches!(
        registry.insert(overflow, 100),
        Err(ReviewRegistryError::CapacityExceeded)
    ));
    assert_eq!(registry.purge_expired(200), 1);
    assert!(registry.is_empty());

    let reviewed = plan(
        postgres_target(),
        vec![MutationChange::DeleteRow {
            locator: vec![field("id", OwnedValue::unsigned(4))],
        }],
    )
    .review(review_id_with(32), 200, 300)
    .unwrap();
    registry.insert(reviewed, 200).unwrap();
    assert!(registry.revoke(review_id_with(32)));
    assert!(!registry.revoke(review_id_with(32)));
}
