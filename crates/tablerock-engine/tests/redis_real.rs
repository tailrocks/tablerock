use std::{collections::BTreeSet, time::Duration};

use redis::AsyncCommands;
use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, Engine, IdParts, PageDelivery, PageIdentity, PageLimits,
    PageWarning, ResultId, Revision, Truncation, ValueKind,
};
use tablerock_engine::{RedisConnectConfig, RedisProtocol, RedisSession, RedisTlsMode};
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
    PageIdentity::new(
        ResultId::from_parts(IdParts::new(0, 2).unwrap()).unwrap(),
        Revision::INITIAL,
        Engine::Redis,
    )
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

        let mut stream = session
            .scan_keys(PageLimits::new(2, 1, 256, 64), 128, 2, 128)
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
    }
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
