use tablerock_core::{
    BoundedText, ByteLimit, CatalogBuildError, CatalogChildrenState, CatalogCursor,
    CatalogIdentity, CatalogLimits, CatalogNode, CatalogNodeId, CatalogNodeKind, CatalogRejection,
    CatalogSnapshot, ClickHouseObjectKind, ContextId, Engine, EngineType, FailureClass, IdParts,
    OperationSafety, OperationScope, OutcomeCertainty, PostgreSqlObjectKind, ProfileId,
    RedisKeyKind, Revision, SafeDiagnostic, SessionId, Severity,
};

fn opaque<T>(
    low: u64,
    build: impl FnOnce(IdParts) -> Result<T, tablerock_core::IdDecodeError>,
) -> T {
    build(IdParts::new(0, low).unwrap()).unwrap()
}

fn node_id(low: u64) -> CatalogNodeId {
    opaque(low, CatalogNodeId::from_parts)
}

fn scope(seed: u64) -> OperationScope {
    OperationScope::new(
        opaque(seed, ProfileId::from_parts),
        opaque(seed + 1, SessionId::from_parts),
        opaque(seed + 2, ContextId::from_parts),
    )
}

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(256)).unwrap()
}

fn node(
    id: u64,
    parent: Option<u64>,
    depth: u16,
    kind: CatalogNodeKind,
    name: &str,
    children: CatalogChildrenState,
) -> CatalogNode {
    CatalogNode::new(
        node_id(id),
        parent.map(node_id),
        depth,
        kind,
        text(name),
        None,
        children,
    )
}

fn identity(engine: Engine, revision: u64) -> CatalogIdentity {
    CatalogIdentity::new(scope(10), engine, Revision::from_wire_u64(revision))
}

fn limits() -> CatalogLimits {
    CatalogLimits::new(20_000, 32, 1_000_000).unwrap()
}

fn read_failure(engine: Engine) -> SafeDiagnostic {
    SafeDiagnostic::new(
        FailureClass::Connectivity,
        engine,
        Severity::Error,
        OutcomeCertainty::ReadOnly,
        OperationSafety::ProvenReadOnly,
    )
}

#[test]
fn accepts_engine_native_lazy_hierarchies() {
    let postgres = CatalogSnapshot::new(
        identity(Engine::PostgreSql, 1),
        vec![
            node(
                1,
                None,
                0,
                CatalogNodeKind::PostgreSqlDatabase,
                "app",
                CatalogChildrenState::Loaded { complete: true },
            ),
            node(
                2,
                Some(1),
                1,
                CatalogNodeKind::PostgreSqlSchema,
                "public",
                CatalogChildrenState::Loaded { complete: true },
            ),
            node(
                3,
                Some(2),
                2,
                CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::Table),
                "users",
                CatalogChildrenState::Loading,
            ),
            CatalogNode::new(
                node_id(4),
                Some(node_id(3)),
                3,
                CatalogNodeKind::PostgreSqlColumn,
                text("email"),
                Some(EngineType::new(Engine::PostgreSql, text("text")).unwrap()),
                CatalogChildrenState::NotApplicable,
            ),
        ],
        limits(),
    )
    .unwrap();
    assert_eq!(postgres.nodes().len(), 4);
    assert_eq!(postgres.text_bytes(), 23);
    assert_eq!(postgres.nodes()[3].name(), "email");
    assert_eq!(postgres.nodes()[3].engine_type().unwrap().name(), "text");

    let clickhouse = CatalogSnapshot::new(
        identity(Engine::ClickHouse, 1),
        vec![
            node(
                10,
                None,
                0,
                CatalogNodeKind::ClickHouseDatabase,
                "analytics",
                CatalogChildrenState::Stale,
            ),
            node(
                11,
                Some(10),
                1,
                CatalogNodeKind::ClickHouseObject(ClickHouseObjectKind::MaterializedView),
                "daily",
                CatalogChildrenState::Unrequested,
            ),
        ],
        limits(),
    )
    .unwrap();
    assert_eq!(
        clickhouse.nodes()[0].children(),
        CatalogChildrenState::Stale
    );

    let redis = CatalogSnapshot::new(
        identity(Engine::Redis, 1),
        vec![
            node(
                20,
                None,
                0,
                CatalogNodeKind::RedisLogicalDatabase,
                "db0",
                CatalogChildrenState::Loaded { complete: false },
            ),
            node(
                21,
                Some(20),
                1,
                CatalogNodeKind::RedisNamespace,
                "tenant",
                CatalogChildrenState::Failed,
            )
            .with_diagnostic(read_failure(Engine::Redis)),
            node(
                22,
                Some(21),
                2,
                CatalogNodeKind::RedisKey(RedisKeyKind::Hash),
                "profile",
                CatalogChildrenState::NotApplicable,
            ),
        ],
        limits(),
    )
    .unwrap();
    assert_eq!(
        redis.nodes()[2].kind(),
        CatalogNodeKind::RedisKey(RedisKeyKind::Hash)
    );
}

#[test]
fn rejects_fake_cross_engine_and_malformed_tree_shapes() {
    let postgres_identity = identity(Engine::PostgreSql, 1);
    let cases = [
        (
            vec![node(
                1,
                None,
                0,
                CatalogNodeKind::ClickHouseDatabase,
                "wrong",
                CatalogChildrenState::Unrequested,
            )],
            CatalogBuildError::EngineMismatch { node: 0 },
        ),
        (
            vec![node(
                1,
                None,
                1,
                CatalogNodeKind::PostgreSqlDatabase,
                "root",
                CatalogChildrenState::Unrequested,
            )],
            CatalogBuildError::InvalidRoot { node: 0 },
        ),
        (
            vec![node(
                2,
                Some(1),
                1,
                CatalogNodeKind::PostgreSqlSchema,
                "orphan",
                CatalogChildrenState::Unrequested,
            )],
            CatalogBuildError::ParentNotBeforeChild { node: 0 },
        ),
        (
            vec![
                node(
                    1,
                    None,
                    0,
                    CatalogNodeKind::PostgreSqlDatabase,
                    "root",
                    CatalogChildrenState::Unrequested,
                ),
                node(
                    2,
                    Some(1),
                    1,
                    CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::Table),
                    "table",
                    CatalogChildrenState::Unrequested,
                ),
            ],
            CatalogBuildError::InvalidHierarchy { node: 1 },
        ),
        (
            vec![
                node(
                    1,
                    None,
                    0,
                    CatalogNodeKind::PostgreSqlDatabase,
                    "root",
                    CatalogChildrenState::Unrequested,
                ),
                node(
                    1,
                    None,
                    0,
                    CatalogNodeKind::PostgreSqlDatabase,
                    "duplicate",
                    CatalogChildrenState::Unrequested,
                ),
            ],
            CatalogBuildError::DuplicateId { node: 1 },
        ),
    ];
    for (nodes, expected) in cases {
        assert_eq!(
            CatalogSnapshot::new(postgres_identity, nodes, limits()),
            Err(expected)
        );
    }
}

#[test]
fn rejects_non_preorder_reentry_leaf_children_and_wrong_type_ownership() {
    let identity = identity(Engine::PostgreSql, 1);
    let root = node(
        1,
        None,
        0,
        CatalogNodeKind::PostgreSqlDatabase,
        "root",
        CatalogChildrenState::Unrequested,
    );
    let first_schema = node(
        2,
        Some(1),
        1,
        CatalogNodeKind::PostgreSqlSchema,
        "one",
        CatalogChildrenState::Unrequested,
    );
    let second_schema = node(
        3,
        Some(1),
        1,
        CatalogNodeKind::PostgreSqlSchema,
        "two",
        CatalogChildrenState::Unrequested,
    );
    let late_child = node(
        4,
        Some(2),
        2,
        CatalogNodeKind::PostgreSqlObject(PostgreSqlObjectKind::View),
        "late",
        CatalogChildrenState::Unrequested,
    );
    assert_eq!(
        CatalogSnapshot::new(
            identity,
            vec![root.clone(), first_schema, second_schema, late_child],
            limits()
        ),
        Err(CatalogBuildError::ParentOutsideActivePath { node: 3 })
    );

    let bad_leaf = node(
        5,
        Some(2),
        2,
        CatalogNodeKind::PostgreSqlColumn,
        "leaf",
        CatalogChildrenState::Loading,
    );
    assert_eq!(
        CatalogSnapshot::new(identity, vec![root.clone(), bad_leaf], limits()),
        Err(CatalogBuildError::InvalidLeafState { node: 1 })
    );

    let typed_root = CatalogNode::new(
        node_id(6),
        None,
        0,
        CatalogNodeKind::PostgreSqlDatabase,
        text("typed-root"),
        Some(EngineType::new(Engine::PostgreSql, text("text")).unwrap()),
        CatalogChildrenState::Unrequested,
    );
    assert_eq!(
        CatalogSnapshot::new(identity, vec![typed_root], limits()),
        Err(CatalogBuildError::InvalidEngineType { node: 0 })
    );

    let failed_without_diagnostic = node(
        7,
        None,
        0,
        CatalogNodeKind::PostgreSqlDatabase,
        "failed",
        CatalogChildrenState::Failed,
    );
    assert_eq!(
        CatalogSnapshot::new(identity, vec![failed_without_diagnostic], limits()),
        Err(CatalogBuildError::InvalidDiagnostic { node: 0 })
    );
}

#[test]
fn enforces_node_depth_and_aggregate_text_limits_before_acceptance() {
    assert_eq!(
        CatalogLimits::new(0, 1, 1),
        Err(CatalogBuildError::InvalidLimits)
    );
    assert_eq!(
        CatalogSnapshot::new(
            identity(Engine::Redis, 1),
            vec![
                node(
                    1,
                    None,
                    0,
                    CatalogNodeKind::RedisLogicalDatabase,
                    "db0",
                    CatalogChildrenState::Unrequested,
                ),
                node(
                    2,
                    None,
                    0,
                    CatalogNodeKind::RedisLogicalDatabase,
                    "db1",
                    CatalogChildrenState::Unrequested,
                ),
            ],
            CatalogLimits::new(1, 1, 16).unwrap(),
        ),
        Err(CatalogBuildError::NodeLimitExceeded {
            actual: 2,
            limit: 1
        })
    );
    assert_eq!(
        CatalogSnapshot::new(
            identity(Engine::Redis, 1),
            vec![node(
                1,
                None,
                0,
                CatalogNodeKind::RedisLogicalDatabase,
                "db0",
                CatalogChildrenState::Unrequested,
            )],
            CatalogLimits::new(1, 0, 2).unwrap(),
        ),
        Err(CatalogBuildError::TextLimitExceeded {
            actual: 3,
            limit: 2
        })
    );
}

#[test]
fn validates_large_synthetic_catalog_and_redacts_names_from_debug() {
    let mut nodes = Vec::with_capacity(10_001);
    nodes.push(node(
        1,
        None,
        0,
        CatalogNodeKind::RedisLogicalDatabase,
        "do-not-log-root",
        CatalogChildrenState::Loaded { complete: true },
    ));
    for index in 0..10_000_u64 {
        nodes.push(node(
            index + 2,
            Some(1),
            1,
            CatalogNodeKind::RedisKey(RedisKeyKind::Unknown),
            "do-not-log-key",
            CatalogChildrenState::NotApplicable,
        ));
    }
    let snapshot = CatalogSnapshot::new(identity(Engine::Redis, 1), nodes, limits()).unwrap();
    assert_eq!(snapshot.nodes().len(), 10_001);
    let debug = format!("{snapshot:?}");
    assert!(!debug.contains("do-not-log"));
    assert!(debug.contains("nodes: 10001"));
}

#[test]
fn cursor_accepts_only_the_next_matching_snapshot_revision() {
    let initial = identity(Engine::ClickHouse, 4);
    let cursor = CatalogCursor::new(initial);
    let snapshot = |identity| CatalogSnapshot::new(identity, Vec::new(), limits()).unwrap();
    assert_eq!(
        cursor.accept(&snapshot(initial)),
        Err(CatalogRejection::StaleOrDuplicate)
    );
    assert_eq!(
        cursor.accept(&snapshot(CatalogIdentity::new(
            scope(99),
            Engine::ClickHouse,
            Revision::from_wire_u64(5),
        ))),
        Err(CatalogRejection::ForeignScope)
    );
    assert_eq!(
        cursor.accept(&snapshot(CatalogIdentity::new(
            scope(10),
            Engine::Redis,
            Revision::from_wire_u64(5),
        ))),
        Err(CatalogRejection::EngineMismatch)
    );
    assert_eq!(
        cursor.accept(&snapshot(identity(Engine::ClickHouse, 6))),
        Err(CatalogRejection::RevisionGap)
    );
    let next = cursor
        .accept(&snapshot(identity(Engine::ClickHouse, 5)))
        .unwrap();
    assert_eq!(next.revision(), Revision::from_wire_u64(5));
}
