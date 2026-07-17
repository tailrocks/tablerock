use std::{collections::BTreeSet, time::Duration};

use redis::{
    AsyncCommands, ConnectionAddr, IntoConnectionInfo, ProtocolVersion, RedisConnectionInfo,
};
use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, CancelDispatch, Engine, PageDelivery, PageIdentity,
    PageLimits, PageWarning, ResultPage, Truncation, ValueKind,
};
use tablerock_engine::{
    AdapterFailureClass, DriverPageRequest, DriverSession, EngineServiceUpdate,
    RedisCollectionScanKind, RedisCollectionScanOptions, RedisConnectConfig, RedisProtocol,
    RedisRuntimePolicy, RedisSession, RedisTlsMode,
};

mod support;
use testcontainers::{
    GenericImage,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(128)).unwrap()
}

fn bytes(value: &[u8]) -> BoundedBytes {
    BoundedBytes::copy_from_slice(value, ByteLimit::new(128)).unwrap()
}

fn identity() -> PageIdentity {
    support::identity(Engine::Redis, 2)
}

#[tokio::test]
async fn scans_binary_keys_and_values_across_supported_redis_matrix() {
    for tag in [
        "7.4.9-alpine@sha256:6ab0b6e7381779332f97b8ca76193e45b0756f38d4c0dcda72dbb3c32061ab99",
        "8.8.0-alpine@sha256:9d317178eceac8454a2284a9e6df2466b93c745529947f0cd42a0fa9609d7005",
    ] {
        verify_version(tag).await;
    }
}

#[tokio::test]
async fn scan_families_preserve_full_iteration_guarantees_during_mutation() {
    for tag in [
        "7.4.9-alpine@sha256:6ab0b6e7381779332f97b8ca76193e45b0756f38d4c0dcda72dbb3c32061ab99",
        "8.8.0-alpine@sha256:9d317178eceac8454a2284a9e6df2466b93c745529947f0cd42a0fa9609d7005",
    ] {
        verify_scan_mutation_version(tag).await;
    }
}

#[tokio::test]
async fn bounds_response_timeouts_and_reconnects_future_reads() {
    for tag in [
        "7.4.9-alpine@sha256:6ab0b6e7381779332f97b8ca76193e45b0756f38d4c0dcda72dbb3c32061ab99",
        "8.8.0-alpine@sha256:9d317178eceac8454a2284a9e6df2466b93c745529947f0cd42a0fa9609d7005",
    ] {
        verify_timeout_reconnect_version(tag).await;
    }
}

#[tokio::test]
async fn bounds_connection_handshake_timeout() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let (_connection, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
    });
    let policy = RedisRuntimePolicy::new(
        Duration::from_millis(100),
        Duration::from_millis(100),
        1,
        Duration::from_millis(1),
        Duration::from_millis(1),
    )
    .unwrap();
    let started = tokio::time::Instant::now();
    let result = RedisSession::connect(
        &RedisConnectConfig::new(
            text("127.0.0.1"),
            port,
            0,
            RedisProtocol::Resp3,
            RedisTlsMode::Disable,
        )
        .with_runtime_policy(policy),
    )
    .await;
    assert!(matches!(result, Err(tablerock_engine::RedisError::Timeout)));
    assert!(started.elapsed() < Duration::from_secs(1));
    server.abort();
}

async fn verify_timeout_reconnect_version(tag: &str) {
    let container = GenericImage::new("redis", tag)
        .with_exposed_port(6379.tcp())
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(6379.tcp()).await.unwrap();
    let policy = RedisRuntimePolicy::new(
        Duration::from_millis(250),
        Duration::from_millis(100),
        8,
        Duration::from_millis(10),
        Duration::from_millis(100),
    )
    .unwrap();

    for protocol in [RedisProtocol::Resp2, RedisProtocol::Resp3] {
        let mut fixture = raw_connection_in_database(port, protocol, 1).await;
        let _: () = redis::cmd("SET")
            .arg(b"reconnect-key")
            .arg(&[0_u8, 255])
            .query_async(&mut fixture)
            .await
            .unwrap();
        let session = RedisSession::connect(
            &RedisConnectConfig::new(text("127.0.0.1"), port, 1, protocol, RedisTlsMode::Disable)
                .with_runtime_policy(policy),
        )
        .await
        .unwrap();

        let _: () = redis::cmd("CLIENT")
            .arg("PAUSE")
            .arg(300_u64)
            .arg("ALL")
            .query_async(&mut fixture)
            .await
            .unwrap();
        assert_eq!(
            session.read_binary(&bytes(b"reconnect-key"), 16).await,
            Err(tablerock_engine::RedisError::Timeout),
            "response timeout {tag} {protocol:?}"
        );
        tokio::time::sleep(Duration::from_millis(350)).await;
        assert!(matches!(
            session
                .read_binary(&bytes(b"reconnect-key"), 16)
                .await
                .unwrap()
                .unwrap()
                .as_ref(),
            tablerock_core::ValueRef::Binary {
                value: [0, 255],
                truncation: Truncation::Complete,
            }
        ));

        let client_id = session.observed_client_id().await.unwrap();
        let killed: u64 = redis::cmd("CLIENT")
            .arg("KILL")
            .arg("ID")
            .arg(client_id)
            .query_async(&mut fixture)
            .await
            .unwrap();
        assert_eq!(killed, 1);
        let first_post_drop = session.read_binary(&bytes(b"reconnect-key"), 16).await;
        if matches!(protocol, RedisProtocol::Resp2) {
            assert_eq!(
                first_post_drop,
                Err(tablerock_engine::RedisError::Connection)
            );
        }
        match first_post_drop {
            Ok(Some(value)) => assert!(matches!(
                value.as_ref(),
                tablerock_core::ValueRef::Binary {
                    value: [0, 255],
                    truncation: Truncation::Complete,
                }
            )),
            Err(tablerock_engine::RedisError::Connection) => {}
            other => panic!("unexpected first post-drop read: {other:?}"),
        }
        let value = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match session.read_binary(&bytes(b"reconnect-key"), 16).await {
                    Ok(Some(value)) => break value,
                    Ok(None) | Err(_) => tokio::time::sleep(Duration::from_millis(25)).await,
                }
            }
        })
        .await
        .expect("future reads reconnect within five seconds");
        assert!(matches!(
            value.as_ref(),
            tablerock_core::ValueRef::Binary {
                value: [0, 255],
                truncation: Truncation::Complete,
            }
        ));
    }
}

async fn verify_scan_mutation_version(tag: &str) {
    let container = GenericImage::new("redis", tag)
        .with_exposed_port(6379.tcp())
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(6379.tcp()).await.unwrap();

    for protocol in [RedisProtocol::Resp2, RedisProtocol::Resp3] {
        let mut mutator = raw_connection(port, protocol).await;
        let stable = seed_scan_race(&mut mutator).await;
        let session = RedisSession::connect(&RedisConnectConfig::new(
            text("127.0.0.1"),
            port,
            0,
            protocol,
            RedisTlsMode::Disable,
        ))
        .await
        .unwrap();

        verify_keyspace_mutation(&session, &mut mutator, &stable, protocol, tag).await;
        for (kind, key) in [
            (RedisCollectionScanKind::Hash, b"race-hash".as_slice()),
            (RedisCollectionScanKind::Set, b"race-set".as_slice()),
            (RedisCollectionScanKind::SortedSet, b"race-zset".as_slice()),
        ] {
            verify_collection_mutation(&session, &mut mutator, &stable, kind, key, protocol, tag)
                .await;
        }
    }
}

async fn raw_connection(port: u16, protocol: RedisProtocol) -> redis::aio::MultiplexedConnection {
    raw_connection_in_database(port, protocol, 0).await
}

async fn raw_connection_in_database(
    port: u16,
    protocol: RedisProtocol,
    database: i64,
) -> redis::aio::MultiplexedConnection {
    let redis = RedisConnectionInfo::default()
        .set_db(database)
        .set_protocol(match protocol {
            RedisProtocol::Resp2 => ProtocolVersion::RESP2,
            RedisProtocol::Resp3 => ProtocolVersion::RESP3,
        });
    let info = ConnectionAddr::Tcp("127.0.0.1".to_owned(), port)
        .into_connection_info()
        .unwrap()
        .set_redis_settings(redis);
    let client = redis::Client::open(info).unwrap();
    tokio::time::timeout(Duration::from_millis(2_500), async {
        loop {
            if let Ok(connection) = client.get_multiplexed_async_connection().await {
                return connection;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("Redis fixture accepts connections within 2.5 seconds")
}

async fn seed_scan_race(connection: &mut redis::aio::MultiplexedConnection) -> BTreeSet<Vec<u8>> {
    let mut stable = (0..600_u16)
        .map(|index| format!("stable-{index:04}").into_bytes())
        .collect::<BTreeSet<_>>();
    stable.insert(vec![0, 255]);
    let mut keys = redis::pipe();
    for member in &stable {
        keys.cmd("SET")
            .arg([b"race-key:".as_slice(), member].concat())
            .arg(1_u8);
    }
    keys.cmd("SET").arg(b"race-key:transient").arg(1_u8);
    keys.cmd("SET").arg(b"race-key:removed-before").arg(1_u8);
    let _: Vec<redis::Value> = keys.query_async(connection).await.unwrap();

    let mut hash = redis::cmd("HSET");
    hash.arg(b"race-hash");
    for member in &stable {
        hash.arg(member).arg(b"value");
    }
    hash.arg(b"transient").arg(b"value");
    hash.arg(b"removed-before").arg(b"value");
    let _: u64 = hash.query_async(connection).await.unwrap();

    let mut set = redis::cmd("SADD");
    set.arg(b"race-set");
    for member in &stable {
        set.arg(member);
    }
    set.arg(b"transient").arg(b"removed-before");
    let _: u64 = set.query_async(connection).await.unwrap();

    let mut sorted_set = redis::cmd("ZADD");
    sorted_set.arg(b"race-zset");
    for (score, member) in stable.iter().enumerate() {
        sorted_set.arg(score).arg(member);
    }
    sorted_set
        .arg(10_000_u64)
        .arg(b"transient")
        .arg(10_001_u64)
        .arg(b"removed-before");
    let _: u64 = sorted_set.query_async(connection).await.unwrap();

    let _: u64 = redis::cmd("DEL")
        .arg(b"race-key:removed-before")
        .arg(b"race-key:late")
        .query_async(connection)
        .await
        .unwrap();
    let _: u64 = redis::cmd("HDEL")
        .arg(b"race-hash")
        .arg(b"removed-before")
        .arg(b"late")
        .query_async(connection)
        .await
        .unwrap();
    let _: u64 = redis::cmd("SREM")
        .arg(b"race-set")
        .arg(b"removed-before")
        .arg(b"late")
        .query_async(connection)
        .await
        .unwrap();
    let _: u64 = redis::cmd("ZREM")
        .arg(b"race-zset")
        .arg(b"removed-before")
        .arg(b"late")
        .query_async(connection)
        .await
        .unwrap();
    stable
}

async fn verify_keyspace_mutation(
    session: &RedisSession,
    mutator: &mut redis::aio::MultiplexedConnection,
    stable: &BTreeSet<Vec<u8>>,
    protocol: RedisProtocol,
    tag: &str,
) {
    let mut stream = session
        .scan_keys(PageLimits::new(8, 1, 4_096, 64), 128, 8, 4_096)
        .unwrap();
    let first = stream.next_page(identity(), 0).await.unwrap().unwrap();
    assert_eq!(first.envelope().delivery(), PageDelivery::Partial);
    let _: u64 = redis::cmd("DEL")
        .arg(b"race-key:transient")
        .query_async(&mut *mutator)
        .await
        .unwrap();
    let _: () = redis::cmd("SET")
        .arg(b"race-key:late")
        .arg(1_u8)
        .query_async(&mut *mutator)
        .await
        .unwrap();
    let mut seen = BTreeSet::new();
    collect_first_column(&first, &mut seen);
    let mut start = u64::from(first.envelope().row_count());
    while let Some(page) = stream.next_page(identity(), start).await.unwrap() {
        collect_first_column(&page, &mut seen);
        start += u64::from(page.envelope().row_count());
    }
    for member in stable {
        assert!(
            seen.contains(&[b"race-key:".as_slice(), member].concat()),
            "stable SCAN key {member:?} {tag} {protocol:?}"
        );
    }
    assert!(!seen.contains(b"race-key:removed-before".as_slice()));
}

async fn verify_collection_mutation(
    session: &RedisSession,
    mutator: &mut redis::aio::MultiplexedConnection,
    stable: &BTreeSet<Vec<u8>>,
    kind: RedisCollectionScanKind,
    key: &[u8],
    protocol: RedisProtocol,
    tag: &str,
) {
    let mut stream = session
        .scan_collection(
            bytes(key),
            kind,
            RedisCollectionScanOptions::new(
                PageLimits::new(8, 2, 4_096, 64),
                128,
                8,
                128,
                65_536,
                4_096,
            ),
        )
        .unwrap();
    let first = stream.next_page(identity(), 0).await.unwrap().unwrap();
    assert_eq!(first.envelope().delivery(), PageDelivery::Partial);
    match kind {
        RedisCollectionScanKind::Hash => {
            let _: u64 = redis::cmd("HDEL")
                .arg(key)
                .arg(b"transient")
                .query_async(&mut *mutator)
                .await
                .unwrap();
            let _: u64 = redis::cmd("HSET")
                .arg(key)
                .arg(b"late")
                .arg(b"value")
                .query_async(&mut *mutator)
                .await
                .unwrap();
        }
        RedisCollectionScanKind::Set => {
            let _: u64 = redis::cmd("SREM")
                .arg(key)
                .arg(b"transient")
                .query_async(&mut *mutator)
                .await
                .unwrap();
            let _: u64 = redis::cmd("SADD")
                .arg(key)
                .arg(b"late")
                .query_async(&mut *mutator)
                .await
                .unwrap();
        }
        RedisCollectionScanKind::SortedSet => {
            let _: u64 = redis::cmd("ZREM")
                .arg(key)
                .arg(b"transient")
                .query_async(&mut *mutator)
                .await
                .unwrap();
            let _: u64 = redis::cmd("ZADD")
                .arg(key)
                .arg(10_002_u64)
                .arg(b"late")
                .query_async(&mut *mutator)
                .await
                .unwrap();
        }
    }
    let mut seen = BTreeSet::new();
    collect_first_column(&first, &mut seen);
    let mut start = u64::from(first.envelope().row_count());
    while let Some(page) = stream.next_page(identity(), start).await.unwrap() {
        collect_first_column(&page, &mut seen);
        start += u64::from(page.envelope().row_count());
    }
    assert!(
        stable.is_subset(&seen),
        "stable {kind:?} members {tag} {protocol:?}"
    );
    assert!(!seen.contains(b"removed-before".as_slice()));
}

fn collect_first_column(page: &ResultPage, values: &mut BTreeSet<Vec<u8>>) {
    assert_ne!(page.envelope().row_count(), 0);
    for row in 0..page.envelope().row_count() {
        values.insert(page.cell(row, 0).unwrap().bytes().to_vec());
    }
}

async fn verify_version(tag: &str) {
    let container = GenericImage::new("redis", tag)
        .with_exposed_port(6379.tcp())
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(6379.tcp()).await.unwrap();
    seed(port).await;

    for protocol in [RedisProtocol::Resp2, RedisProtocol::Resp3] {
        let session = RedisSession::connect(&RedisConnectConfig::new(
            text("127.0.0.1"),
            port,
            0,
            protocol,
            RedisTlsMode::Disable,
        ))
        .await
        .unwrap();
        assert_eq!(session.negotiated_protocol().await.unwrap(), protocol);
        verify_ttl_states(&session, protocol, tag).await;
        verify_hash_scan(&session, protocol, tag).await;
        verify_set_scan(&session, protocol, tag).await;
        verify_sorted_set_scan(&session, protocol, tag).await;
        verify_empty_collection_scans(&session, protocol, tag).await;
        verify_pipeline_partial_failure(port, protocol, tag).await;
        verify_service_cancellation(port, protocol, tag, false).await;
        verify_service_cancellation(port, protocol, tag, true).await;
        verify_blocking_completion(port, protocol).await;

        let value = session
            .read_binary(&bytes(&[0, 255]), 3)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(value.kind(), ValueKind::Binary);
        assert!(matches!(
            value.as_ref(),
            tablerock_core::ValueRef::Binary {
                value: [1, 0, 255],
                truncation: Truncation::Truncated {
                    original_byte_len: Some(4)
                }
            }
        ));

        let driver: &dyn DriverSession = &session;
        assert_eq!(driver.engine(), Engine::Redis);
        let mut stream = driver
            .start_page_stream(DriverPageRequest::RedisKeyScan {
                limits: PageLimits::new(2, 1, 256, 64),
                max_cell_bytes: 128,
                scan_count: 2,
                max_scan_rounds: 128,
            })
            .await
            .unwrap();
        let mut found = BTreeSet::new();
        let mut start = 0_u64;
        while let Some(page) = stream.next_page(identity(), start).await.unwrap() {
            assert_ne!(page.envelope().row_count(), 0);
            for row in 0..page.envelope().row_count() {
                found.insert(page.cell(row, 0).unwrap().bytes().to_vec());
            }
            start += u64::from(page.envelope().row_count());
        }
        assert_eq!(
            found,
            BTreeSet::from([
                vec![0, 255],
                b"long-binary-key".to_vec(),
                b"plain".to_vec(),
                b"scan-hash".to_vec(),
                b"scan-set".to_vec(),
                b"scan-zset".to_vec(),
            ]),
            "Redis {tag} {protocol:?}"
        );

        let mut bounded = session
            .scan_keys(PageLimits::new(8, 1, 4, 64), 2, 8, 128)
            .unwrap();
        let page = bounded.next_page(identity(), 0).await.unwrap().unwrap();
        assert!(
            page.envelope()
                .warnings()
                .contains(PageWarning::ByteLimitReached)
        );
        let stored_bytes: usize = (0..page.envelope().row_count())
            .map(|row| page.cell(row, 0).unwrap().bytes().len())
            .sum();
        assert!(stored_bytes <= 4);

        let isolated = RedisSession::connect(&RedisConnectConfig::new(
            text("127.0.0.1"),
            port,
            1,
            protocol,
            RedisTlsMode::Disable,
        ))
        .await
        .unwrap();
        let mut isolated_scan = isolated
            .scan_keys(PageLimits::new(8, 1, 128, 64), 128, 8, 128)
            .unwrap();
        let page = isolated_scan
            .next_page(identity(), 0)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(page.envelope().delivery(), PageDelivery::Final);
        assert_eq!(page.envelope().row_count(), 1);
        assert_eq!(page.cell(0, 0).unwrap().bytes(), b"database-one");

        drop(stream);
        drop(bounded);
        let operation_id = support::operation(40);
        let mut service = support::service(1, 2);
        service
            .submit(
                operation_id,
                support::command(41),
                Box::new(session),
                DriverPageRequest::RedisKeyScan {
                    limits: PageLimits::new(2, 1, 256, 64),
                    max_cell_bytes: 128,
                    scan_count: 2,
                    max_scan_rounds: 128,
                },
                identity(),
            )
            .await
            .unwrap();
        let mut service_keys = BTreeSet::new();
        loop {
            match service.next_update(operation_id).await.unwrap().unwrap() {
                EngineServiceUpdate::Started => {}
                EngineServiceUpdate::Page(page) => {
                    for row in 0..page.envelope().row_count() {
                        service_keys.insert(page.cell(row, 0).unwrap().bytes().to_vec());
                    }
                }
                EngineServiceUpdate::Terminal(tablerock_core::OperationOutcome::Completed) => {
                    break;
                }
                other => panic!("unexpected Redis service event: {other:?}"),
            }
        }
        assert_eq!(service_keys, found, "Redis service {tag} {protocol:?}");
    }
}

async fn verify_hash_scan(session: &RedisSession, protocol: RedisProtocol, tag: &str) {
    let driver: &dyn DriverSession = session;
    let mut stream = driver
        .start_page_stream(DriverPageRequest::RedisCollectionScan {
            key: bytes(b"scan-hash"),
            kind: RedisCollectionScanKind::Hash,
            options: RedisCollectionScanOptions::new(
                PageLimits::new(1, 2, 64, 128),
                16,
                1,
                16,
                1_024,
                128,
            ),
        })
        .await
        .unwrap();
    let mut rows = BTreeSet::new();
    let mut start = 0_u64;
    while let Some(page) = stream.next_page(identity(), start).await.unwrap() {
        assert!(page.envelope().row_count() <= 1);
        assert_eq!(page.envelope().column_count(), 2);
        for row in 0..page.envelope().row_count() {
            rows.insert((
                page.cell(row, 0).unwrap().bytes().to_vec(),
                page.cell(row, 1).unwrap().bytes().to_vec(),
            ));
        }
        start += u64::from(page.envelope().row_count());
    }
    assert_eq!(
        rows,
        BTreeSet::from([
            (vec![0, 255], vec![1, 2, 3, 4]),
            (b"field".to_vec(), b"value".to_vec()),
        ]),
        "HSCAN {tag} {protocol:?}"
    );

    let mut oversized = driver
        .start_page_stream(DriverPageRequest::RedisCollectionScan {
            key: bytes(b"scan-hash"),
            kind: RedisCollectionScanKind::Hash,
            options: RedisCollectionScanOptions::new(
                PageLimits::new(2, 2, 64, 128),
                16,
                1,
                1,
                1_024,
                128,
            ),
        })
        .await
        .unwrap();
    let error = oversized.next_page(identity(), 0).await.unwrap_err();
    assert_eq!(
        error.class(),
        AdapterFailureClass::ResourceLimit,
        "bounded HSCAN response {tag} {protocol:?}"
    );
}

async fn verify_set_scan(session: &RedisSession, protocol: RedisProtocol, tag: &str) {
    let mut stream = session
        .scan_collection(
            bytes(b"scan-set"),
            RedisCollectionScanKind::Set,
            RedisCollectionScanOptions::new(PageLimits::new(1, 1, 32, 128), 16, 1, 16, 1_024, 128),
        )
        .unwrap();
    let mut members = BTreeSet::new();
    let mut start = 0_u64;
    while let Some(page) = stream.next_page(identity(), start).await.unwrap() {
        assert!(page.envelope().row_count() <= 1);
        assert_eq!(page.envelope().column_count(), 1);
        for row in 0..page.envelope().row_count() {
            members.insert(page.cell(row, 0).unwrap().bytes().to_vec());
        }
        start += u64::from(page.envelope().row_count());
    }
    assert_eq!(
        members,
        BTreeSet::from([vec![0, 255], b"member".to_vec()]),
        "SSCAN {tag} {protocol:?}"
    );
}

async fn verify_sorted_set_scan(session: &RedisSession, protocol: RedisProtocol, tag: &str) {
    let mut stream = session
        .scan_collection(
            bytes(b"scan-zset"),
            RedisCollectionScanKind::SortedSet,
            RedisCollectionScanOptions::new(PageLimits::new(1, 2, 32, 128), 16, 1, 16, 1_024, 128),
        )
        .unwrap();
    let mut members = BTreeSet::new();
    let mut start = 0_u64;
    while let Some(page) = stream.next_page(identity(), start).await.unwrap() {
        assert!(page.envelope().row_count() <= 1);
        assert_eq!(page.envelope().column_count(), 2);
        for row in 0..page.envelope().row_count() {
            let score_cell = page.cell(row, 1).unwrap();
            assert_eq!(score_cell.kind(), ValueKind::Float64);
            let score = u64::from_be_bytes(score_cell.bytes().try_into().unwrap());
            members.insert((page.cell(row, 0).unwrap().bytes().to_vec(), score));
        }
        start += u64::from(page.envelope().row_count());
    }
    assert_eq!(
        members,
        BTreeSet::from([
            (vec![0, 255], (-1.25_f64).to_bits()),
            (b"member".to_vec(), 2.5_f64.to_bits()),
        ]),
        "ZSCAN {tag} {protocol:?}"
    );
}

async fn verify_empty_collection_scans(session: &RedisSession, protocol: RedisProtocol, tag: &str) {
    for (kind, columns) in [
        (RedisCollectionScanKind::Hash, 2),
        (RedisCollectionScanKind::Set, 1),
        (RedisCollectionScanKind::SortedSet, 2),
    ] {
        let mut stream = session
            .scan_collection(
                bytes(b"missing-collection"),
                kind,
                RedisCollectionScanOptions::new(
                    PageLimits::new(1, columns, 16, 128),
                    8,
                    1,
                    8,
                    128,
                    8,
                ),
            )
            .unwrap();
        let page = stream.next_page(identity(), 0).await.unwrap().unwrap();
        assert_eq!(
            page.envelope().row_count(),
            0,
            "empty {kind:?} {tag} {protocol:?}"
        );
        assert_eq!(page.envelope().column_count(), columns);
        assert_eq!(page.envelope().delivery(), PageDelivery::Final);
        assert!(stream.next_page(identity(), 0).await.unwrap().is_none());
    }
}

async fn verify_ttl_states(session: &RedisSession, protocol: RedisProtocol, tag: &str) {
    assert_eq!(
        session.read_time_to_live(&bytes(&[0, 1])).await.unwrap(),
        tablerock_core::RedisTimeToLive::Missing,
        "missing TTL {tag} {protocol:?}"
    );
    assert_eq!(
        session.read_time_to_live(&bytes(b"plain")).await.unwrap(),
        tablerock_core::RedisTimeToLive::Persistent,
        "persistent TTL {tag} {protocol:?}"
    );

    let expiring = session
        .read_time_to_live(&bytes(b"long-binary-key"))
        .await
        .unwrap();
    assert!(
        matches!(
            expiring,
            tablerock_core::RedisTimeToLive::Expiring {
                remaining_millis: 1..=600_000
            }
        ),
        "finite TTL {tag} {protocol:?}: {expiring:?}"
    );
}

#[derive(Debug, Clone, Copy)]
enum PipelineMode {
    Pipelined,
    MultiExec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PipelineOutcome {
    ServerSucceeded,
    ServerRejected,
}

async fn verify_pipeline_partial_failure(port: u16, protocol: RedisProtocol, tag: &str) {
    for mode in [PipelineMode::Pipelined, PipelineMode::MultiExec] {
        let redis = RedisConnectionInfo::default()
            .set_db(0)
            .set_protocol(match protocol {
                RedisProtocol::Resp2 => ProtocolVersion::RESP2,
                RedisProtocol::Resp3 => ProtocolVersion::RESP3,
            });
        let info = ConnectionAddr::Tcp("127.0.0.1".to_owned(), port)
            .into_connection_info()
            .unwrap()
            .set_redis_settings(redis);
        let client = redis::Client::open(info).unwrap();
        let mut connection = client.get_multiplexed_async_connection().await.unwrap();
        let key = format!("tablerock-pipeline-{tag}-{protocol:?}-{mode:?}");
        let mut pipeline = redis::pipe();
        if matches!(mode, PipelineMode::MultiExec) {
            pipeline.atomic();
        }
        pipeline
            .cmd("SET")
            .arg(key.as_bytes())
            .arg(1_u8)
            .cmd("HSET")
            .arg(key.as_bytes())
            .arg(b"field")
            .arg(b"value")
            .cmd("INCR")
            .arg(key.as_bytes())
            .ignore_errors();
        let results: Vec<redis::RedisResult<redis::Value>> =
            pipeline.query_async(&mut connection).await.unwrap();
        let outcomes = results
            .iter()
            .map(|result| {
                if result.is_ok() {
                    PipelineOutcome::ServerSucceeded
                } else {
                    PipelineOutcome::ServerRejected
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            outcomes,
            &[
                PipelineOutcome::ServerSucceeded,
                PipelineOutcome::ServerRejected,
                PipelineOutcome::ServerSucceeded,
            ],
            "Redis pipeline outcomes {tag} {protocol:?} {mode:?}"
        );
        let final_counter: u64 = redis::cmd("GET")
            .arg(key.as_bytes())
            .query_async(&mut connection)
            .await
            .unwrap();
        assert_eq!(
            final_counter, 2,
            "Redis does not roll back successful commands {tag} {protocol:?} {mode:?}"
        );
        redis::cmd("DEL")
            .arg(key.as_bytes())
            .exec_async(&mut connection)
            .await
            .unwrap();
    }
}

async fn verify_service_cancellation(
    port: u16,
    protocol: RedisProtocol,
    tag: &str,
    wait_for_server_dispatch: bool,
) {
    let session = RedisSession::connect(&RedisConnectConfig::new(
        text("127.0.0.1"),
        port,
        0,
        protocol,
        RedisTlsMode::Disable,
    ))
    .await
    .unwrap();
    let operation_id = support::operation(60);
    let mut service = support::service(1, 2);
    service
        .submit(
            operation_id,
            support::command(61),
            Box::new(session),
            DriverPageRequest::RedisBlockingPop {
                key: bytes(b"tablerock-cancellation-empty-list"),
                limits: PageLimits::new(1, 2, 256, 128),
                max_cell_bytes: 128,
            },
            identity(),
        )
        .await
        .unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(5), service.next_update(operation_id))
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        EngineServiceUpdate::Started
    ));
    if wait_for_server_dispatch {
        wait_until_blocked(port, None).await;
    }
    let cancel = service.cancel(operation_id).unwrap();
    assert_eq!(cancel.core, tablerock_core::CancelRequestOutcome::Requested);
    assert_eq!(
        cancel.runtime,
        Some(tablerock_engine::RuntimeCancelOutcome::Queued)
    );
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(5), service.next_update(operation_id))
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        EngineServiceUpdate::CancelDispatched(dispatch)
            if dispatch == if wait_for_server_dispatch {
                CancelDispatch::RequestSent
            } else {
                CancelDispatch::PreventedBeforeDispatch
            }
    ));
    assert!(
        matches!(
            tokio::time::timeout(Duration::from_secs(5), service.next_update(operation_id))
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            EngineServiceUpdate::Terminal(outcome)
                if outcome == if wait_for_server_dispatch {
                    tablerock_core::OperationOutcome::ServerConfirmedCancelled
                } else {
                    tablerock_core::OperationOutcome::ClientStopped
                }
        ),
        "Redis cancellation outcome {tag} {protocol:?}"
    );
}

async fn wait_until_blocked(port: u16, client_id: Option<u64>) {
    let client = redis::Client::open(format!("redis://127.0.0.1:{port}/0")).unwrap();
    let mut inspector = client.get_multiplexed_async_connection().await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let mut command = redis::cmd("CLIENT");
            command.arg("LIST");
            if let Some(client_id) = client_id {
                command.arg("ID").arg(client_id);
            }
            let state: String = command.query_async(&mut inspector).await.unwrap();
            if state.split_whitespace().any(|field| field == "flags=b") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Redis reports the operation connection as blocked");
}

async fn verify_blocking_completion(port: u16, protocol: RedisProtocol) {
    let session = RedisSession::connect(&RedisConnectConfig::new(
        text("127.0.0.1"),
        port,
        0,
        protocol,
        RedisTlsMode::Disable,
    ))
    .await
    .unwrap();
    let mut stream = session
        .blocking_pop(
            bytes(b"tablerock-blocking-completion"),
            PageLimits::new(1, 2, 256, 128),
            128,
        )
        .await
        .unwrap();
    let client_id = session.active_blocking_client_id().unwrap();
    assert!(matches!(
        session
            .blocking_pop(
                bytes(b"second-blocking-operation"),
                PageLimits::new(1, 2, 256, 128),
                128,
            )
            .await,
        Err(tablerock_engine::RedisError::SessionBusy)
    ));
    wait_until_blocked(port, Some(client_id)).await;
    let client = redis::Client::open(format!("redis://127.0.0.1:{port}/0")).unwrap();
    let mut producer = client.get_multiplexed_async_connection().await.unwrap();
    let pushed: u64 = redis::cmd("RPUSH")
        .arg(b"tablerock-blocking-completion")
        .arg(&[0_u8, 255])
        .query_async(&mut producer)
        .await
        .unwrap();
    assert_eq!(pushed, 1);
    let page = tokio::time::timeout(Duration::from_secs(5), stream.next_page(identity(), 0))
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(session.active_blocking_client_id(), None);
    assert_eq!(page.envelope().delivery(), PageDelivery::Final);
    assert_eq!(
        page.cell(0, 0).unwrap().bytes(),
        b"tablerock-blocking-completion"
    );
    assert_eq!(page.cell(0, 1).unwrap().bytes(), &[0, 255]);
}

async fn seed(port: u16) {
    for (database, entries) in [
        (
            0,
            vec![
                (vec![0, 255], vec![1, 0, 255, 2]),
                (b"long-binary-key".to_vec(), b"value".to_vec()),
                (b"plain".to_vec(), b"value".to_vec()),
            ],
        ),
        (1, vec![(b"database-one".to_vec(), b"isolated".to_vec())]),
    ] {
        let client = redis::Client::open(format!("redis://127.0.0.1:{port}/{database}")).unwrap();
        let mut connection = None;
        for _ in 0..50 {
            match client.get_multiplexed_async_connection().await {
                Ok(connected) => {
                    connection = Some(connected);
                    break;
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(20)).await,
            }
        }
        let mut connection = connection.expect("Redis fixture accepts connections");
        for (key, value) in entries {
            let _: () = connection.set(key, value).await.unwrap();
        }
        if database == 0 {
            let _: bool = redis::cmd("PEXPIRE")
                .arg(b"long-binary-key")
                .arg(600_000_u64)
                .query_async(&mut connection)
                .await
                .unwrap();
            let _: u64 = redis::cmd("HSET")
                .arg(b"scan-hash")
                .arg(&[0_u8, 255])
                .arg(&[1_u8, 2, 3, 4])
                .arg(b"field")
                .arg(b"value")
                .query_async(&mut connection)
                .await
                .unwrap();
            let _: u64 = redis::cmd("SADD")
                .arg(b"scan-set")
                .arg(&[0_u8, 255])
                .arg(b"member")
                .query_async(&mut connection)
                .await
                .unwrap();
            let _: u64 = redis::cmd("ZADD")
                .arg(b"scan-zset")
                .arg(-1.25_f64)
                .arg(&[0_u8, 255])
                .arg(2.5_f64)
                .arg(b"member")
                .query_async(&mut connection)
                .await
                .unwrap();
        }
    }
}
