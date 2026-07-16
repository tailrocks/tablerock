use std::{collections::BTreeSet, time::Duration};

use redis::{
    AsyncCommands, ConnectionAddr, IntoConnectionInfo, ProtocolVersion, RedisConnectionInfo,
};
use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, CancelDispatch, Engine, PageDelivery, PageIdentity,
    PageLimits, PageWarning, Truncation, ValueKind,
};
use tablerock_engine::{
    DriverPageRequest, DriverSession, EngineServiceUpdate, RedisConnectConfig, RedisProtocol,
    RedisSession, RedisTlsMode,
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
        verify_pipeline_partial_failure(port, protocol, tag).await;
        verify_service_cancellation(port, protocol, tag).await;
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
            BTreeSet::from([vec![0, 255], b"long-binary-key".to_vec(), b"plain".to_vec()]),
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

async fn verify_service_cancellation(port: u16, protocol: RedisProtocol, tag: &str) {
    let session = RedisSession::connect(&RedisConnectConfig::new(
        text("127.0.0.1"),
        port,
        0,
        protocol,
        RedisTlsMode::Disable,
    ))
    .await
    .unwrap();
    let blocked_client_id = session.client_id();
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
    wait_until_blocked(port, blocked_client_id).await;
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
        EngineServiceUpdate::CancelDispatched(CancelDispatch::RequestSent)
    ));
    assert!(
        matches!(
            tokio::time::timeout(Duration::from_secs(5), service.next_update(operation_id))
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            EngineServiceUpdate::Terminal(
                tablerock_core::OperationOutcome::ServerConfirmedCancelled
            )
        ),
        "Redis cancellation outcome {tag} {protocol:?}"
    );
}

async fn wait_until_blocked(port: u16, client_id: u64) {
    let client = redis::Client::open(format!("redis://127.0.0.1:{port}/0")).unwrap();
    let mut inspector = client.get_multiplexed_async_connection().await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let state: String = redis::cmd("CLIENT")
                .arg("LIST")
                .arg("ID")
                .arg(client_id)
                .query_async(&mut inspector)
                .await
                .unwrap();
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
    let client_id = session.client_id();
    let mut stream = session
        .blocking_pop(
            bytes(b"tablerock-blocking-completion"),
            PageLimits::new(1, 2, 256, 128),
            128,
        )
        .unwrap();
    assert!(matches!(
        session.blocking_pop(
            bytes(b"second-blocking-operation"),
            PageLimits::new(1, 2, 256, 128),
            128,
        ),
        Err(tablerock_engine::RedisError::SessionBusy)
    ));
    wait_until_blocked(port, client_id).await;
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
    }
}
