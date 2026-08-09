//! Honest product path: prepare → save → open_profile (as-draft) → catalog → execute.
//!
//! Uses `TABLEROCK_TEST_ROOT` so default path resolution and prepare share one root.
//! Does **not** rewrite host/database to absolute paths before open.

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use tablerock_core::{
    SAMPLE_DATABASE_PROFILE_NAME, SAMPLE_STARTER_SQL, sample_sqlite_database_path, ResultPage,
    PageLimits,
};
use tablerock_engine::ensure_sample_sqlite_database;
use tablerock_ffi::{SubmitSpec, TableRockBridge};
use tablerock_persistence::{OPERATOR_PROFILES_DB_FILE, TEST_ROOT_ENV};

fn unique_root() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tablerock-sample-{}-{}",
        std::process::id(),
        nanos
    ))
}

#[tokio::test]
async fn ensure_sample_creates_demo_tables() {
    let root = unique_root();
    fs::create_dir_all(&root).unwrap();
    let path = sample_sqlite_database_path(&root);
    ensure_sample_sqlite_database(&path).await.unwrap();
    assert!(path.is_file());
    let session = tablerock_engine::SqliteSession::connect(&path).await.unwrap();
    use tablerock_engine::{CatalogRequest, DriverSession};
    let subtree = session
        .catalog(CatalogRequest::SqliteTables {
            limits: tablerock_core::PageLimits::new(100, 32, 1024 * 1024, 64 * 1024),
        })
        .await
        .unwrap();
    let names: Vec<_> = subtree.nodes().iter().map(|n| n.name().to_owned()).collect();
    assert!(names.iter().any(|n| n == "artists"), "{names:?}");
    assert!(names.iter().any(|n| n == "tracks"), "{names:?}");
    assert!(names.iter().any(|n| n == "orders"), "{names:?}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn prepare_sample_via_bridge_save_and_open() {
    let root = unique_root();
    fs::create_dir_all(&root).unwrap();
    // Product isolation: default operator root + prepare data_root must agree.
    // SAFETY: test-only env; single-threaded test process for this binary.
    unsafe {
        std::env::set_var(TEST_ROOT_ENV, &root);
    }
    let db = root.join(OPERATOR_PROFILES_DB_FILE);
    let bridge = TableRockBridge::new_for_test();
    bridge
        .configure_persistence(db.to_string_lossy().into_owned())
        .unwrap();
    let draft = bridge
        .prepare_sample_database(root.to_string_lossy().into_owned())
        .unwrap();
    assert_eq!(draft.name, SAMPLE_DATABASE_PROFILE_NAME);
    assert_eq!(draft.engine, "sqlite");
    assert_eq!(draft.host, "local");
    assert_eq!(draft.database, "samples/tablerock-sample.db");
    assert_eq!(draft.password_source, "none");
    assert!(sample_sqlite_database_path(&root).is_file());

    // Save as-draft — no host rewrite.
    let id = bridge.save_profile(draft).unwrap();
    let listed = bridge.list_profiles().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, SAMPLE_DATABASE_PROFILE_NAME);
    assert_eq!(listed[0].engine, "sqlite");

    // Round-trip: reloaded draft must keep passwordless source (native connect
    // prompts only when password_source == "prompt").
    let reloaded = bridge
        .get_profile_draft(id.clone())
        .expect("get_profile_draft after sample save");
    assert_eq!(
        reloaded.password_source, "none",
        "sample must stay passwordless after save; got {:?}",
        reloaded.password_source
    );
    assert_eq!(reloaded.engine, "sqlite");
    assert_eq!(reloaded.host, "local");
    assert_eq!(reloaded.database, "samples/tablerock-sample.db");

    // Open product draft without secret override (native path after skip prompt).
    let session = bridge
        .open_profile(id.clone(), None)
        .expect("open_profile must resolve sample under persistence/data root without password");
    assert_eq!(session.len(), 16);

    let nodes = bridge.refresh_catalog(session.clone(), None).unwrap();
    assert!(!nodes.is_empty(), "catalog root must be non-empty");
    assert!(
        nodes.iter().any(|n| n.kind.contains("sqlite") || n.kind.contains("database")),
        "expected sqlite database root: {nodes:?}"
    );
    let root_id = nodes[0].id_bytes.clone();
    let tables = bridge
        .refresh_catalog(session.clone(), Some(root_id))
        .expect("expand sqlite root to tables");
    assert!(
        !tables.is_empty(),
        "sample tables must load via product path"
    );
    assert!(
        tables
            .iter()
            .any(|n| n.name == "artists" || n.name == "tracks" || n.name == "orders"),
        "demo tables missing: {tables:?}"
    );

    // Bridge execute starter SQL → non-empty page (not direct SqliteSession).
    let operation = bridge
        .submit(SubmitSpec {
            intent: "execute".into(),
            session_id: session.clone(),
            statement: Some(SAMPLE_STARTER_SQL.into()),
            result_id: None,
            start_row: None,
            row_count: Some(64),
            expected_revision: 0,
        })
        .expect("submit SAMPLE_STARTER_SQL");
    bridge.pump(operation).expect("pump execute");
    let batch = bridge.next_events(0, 64).expect("events");
    let page_bytes = batch
        .events
        .iter()
        .rev()
        .find(|e| e.kind == "page")
        .and_then(|e| e.page_bytes.clone())
        .expect("page event for starter SQL");
    let page = ResultPage::decode_v1(
        &page_bytes,
        PageLimits::new(500, 64, 4 * 1024 * 1024, 64 * 1024),
    )
    .expect("decode page");
    assert!(!page.columns().is_empty(), "starter SQL columns");
    assert!(
        page.envelope().row_count() > 0,
        "starter SQL must return rows"
    );

    let _ = bridge.shutdown(false, 1_000);
    unsafe {
        std::env::remove_var(TEST_ROOT_ENV);
    }
    fs::remove_dir_all(&root).ok();
}
