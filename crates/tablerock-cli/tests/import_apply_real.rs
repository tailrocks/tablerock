//! Real-path CSV import apply: parse → InsertRow plan → authorize → PG apply.

use std::sync::Arc;

use tablerock_cli::{apply_csv_inserts, parse_csv};
use tablerock_core::{
    BoundedText, ByteLimit, ContextId, IdParts, MutationId, MutationTarget, OperationScope,
    ProfileId, ReviewTokenId, Revision, SessionId,
};
use tablerock_engine::{
    MutationTransactionState, PostgresConnectConfig, PostgresSession, PostgresTlsMode,
};
use testcontainers::{
    GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

fn bt(s: &str) -> BoundedText {
    BoundedText::copy_from_str(s, ByteLimit::new(128)).unwrap()
}

#[ignore = "real-server test: runs in CI real-servers job with --include-ignored"]
#[tokio::test]
async fn applies_csv_insert_rows_through_mutation_seam() {
    let container = GenericImage::new("postgres", "18.4-alpine")
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(5432.tcp()).await.unwrap();
    let host = container.get_host().await.unwrap().to_string();
    let session = PostgresSession::connect(&PostgresConnectConfig::new(
        bt(&host),
        port,
        bt("postgres"),
        bt("postgres"),
        PostgresTlsMode::Disabled,
    ))
    .await
    .unwrap();
    session
        .execute_sql(
            "CREATE TABLE csv_import_probe (
                id text PRIMARY KEY,
                label text NOT NULL
             )",
        )
        .await
        .unwrap();

    let csv = "id,label\n1,hello\n2,=SUM(A1)\n";
    let table = parse_csv(csv, 100, 256).unwrap();
    let scope = OperationScope::new(
        ProfileId::from_parts(IdParts::new(1, 1).unwrap()).unwrap(),
        SessionId::from_parts(IdParts::new(1, 2).unwrap()).unwrap(),
        ContextId::from_parts(IdParts::new(1, 3).unwrap()).unwrap(),
    );
    let target = MutationTarget::PostgreSqlRelation {
        database: bt("postgres"),
        schema: bt("public"),
        relation: bt("csv_import_probe"),
    };
    let mutation_id = MutationId::from_parts(IdParts::new(1, 10).unwrap()).unwrap();
    let token = ReviewTokenId::from_parts(IdParts::new(1, 11).unwrap()).unwrap();
    let outcome = apply_csv_inserts(
        Arc::new(session),
        &table,
        target,
        scope,
        Revision::INITIAL,
        mutation_id,
        token,
        256,
        16,
        1_000,
        30_000,
    )
    .await
    .expect("import apply");

    assert_eq!(outcome.transaction, MutationTransactionState::Committed);
    assert_eq!(outcome.changes.len(), 2);
    assert!(
        outcome.changes.iter().all(|c| matches!(
            c,
            tablerock_engine::MutationChangeOutcome::Applied {
                rows_affected: 1,
                ..
            }
        )),
        "changes: {:?}",
        outcome.changes
    );
}

#[ignore = "real-server test: runs in CI real-servers job with --include-ignored"]
#[tokio::test]
async fn applies_csv_insert_rows_on_clickhouse_progressive() {
    use tablerock_engine::{
        ClickHouseCompression, ClickHouseConnectConfig, ClickHouseSession, ClickHouseTlsMode,
    };

    let image =
        "26.3.17.4-jammy@sha256:158dcce6f6fdc59309650aad6b79484abf4eed07d4e0bdba31d732e64b5a25fb";
    let container = GenericImage::new("clickhouse", image)
        .with_exposed_port(8123.tcp())
        .with_env_var("CLICKHOUSE_SKIP_USER_SETUP", "1")
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(8123.tcp()).await.unwrap();
    let host = container.get_host().await.unwrap().to_string();

    // Wait for HTTP readiness.
    let session = {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            let s = ClickHouseSession::connect(&ClickHouseConnectConfig::new(
                bt(&host),
                port,
                bt("default"),
                bt("default"),
                ClickHouseTlsMode::Disable,
                ClickHouseCompression::Lz4,
            ));
            match s.health_check().await {
                Ok(()) => break s,
                Err(_) if tokio::time::Instant::now() < deadline => {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                Err(e) => panic!("clickhouse not ready: {e:?}"),
            }
        }
    };

    // MergeTree table for progressive inserts.
    session
        .execute_sql(
            "CREATE TABLE IF NOT EXISTS default.csv_import_ch (
                id String,
                label String
             ) ENGINE = MergeTree ORDER BY id",
        )
        .await
        .unwrap();

    let csv = "id,label\n10,hello\n20,=CMD()\n";
    let table = parse_csv(csv, 100, 256).unwrap();
    let scope = OperationScope::new(
        ProfileId::from_parts(IdParts::new(1, 1).unwrap()).unwrap(),
        SessionId::from_parts(IdParts::new(1, 2).unwrap()).unwrap(),
        ContextId::from_parts(IdParts::new(1, 3).unwrap()).unwrap(),
    );
    let target = MutationTarget::ClickHouseTable {
        database: bt("default"),
        table: bt("csv_import_ch"),
    };
    let mutation_id = MutationId::from_parts(IdParts::new(1, 20).unwrap()).unwrap();
    let token = ReviewTokenId::from_parts(IdParts::new(1, 21).unwrap()).unwrap();
    let outcome = apply_csv_inserts(
        Arc::new(session),
        &table,
        target,
        scope,
        Revision::INITIAL,
        mutation_id,
        token,
        256,
        16,
        1_000,
        30_000,
    )
    .await
    .expect("ch import apply");

    assert_eq!(outcome.changes.len(), 2);
    assert!(
        outcome
            .changes
            .iter()
            .all(|c| matches!(c, tablerock_engine::MutationChangeOutcome::Applied { .. })),
        "changes: {:?}",
        outcome.changes
    );
}
