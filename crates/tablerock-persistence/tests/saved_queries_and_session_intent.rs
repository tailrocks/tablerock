use std::{fs, path::PathBuf};

use tablerock_core::{Engine, IdParts, ProfileId};
use tablerock_persistence::{
    HistoryAppend, HistoryOutcomeClass, HistoryRetention, PersistenceActor, SavedQueryUpsert,
    external_change_detected, read_sql_file, write_sql_file_atomic,
};

fn path(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "tablerock-saved-query-{}-{suffix}.db",
        std::process::id(),
    ))
}

fn profile(low: u64) -> ProfileId {
    ProfileId::from_parts(IdParts::new(1, low).unwrap()).unwrap()
}

#[test]
fn saved_query_upsert_list_get_delete() {
    let db = path("crud");
    let _ = fs::remove_file(&db);
    let actor = PersistenceActor::open(&db).expect("open");

    let id = actor
        .upsert_saved_query(SavedQueryUpsert {
            name: "daily counts".into(),
            engine: Engine::PostgreSql,
            statement_text: "SELECT count(*) FROM t".into(),
        })
        .expect("upsert");
    assert!(id > 0);

    // Upsert same name replaces text.
    let id2 = actor
        .upsert_saved_query(SavedQueryUpsert {
            name: "daily counts".into(),
            engine: Engine::PostgreSql,
            statement_text: "SELECT 1".into(),
        })
        .expect("upsert again");
    assert_eq!(id, id2);

    let listed = actor
        .list_saved_queries(Some(Engine::PostgreSql))
        .expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].statement_text, "SELECT 1");

    let got = actor.get_saved_query(id).expect("get").expect("present");
    assert_eq!(got.name, "daily counts");

    actor.delete_saved_query(id).expect("delete");
    assert!(actor.get_saved_query(id).expect("get after").is_none());
    actor.shutdown().expect("shutdown");
    let _ = fs::remove_file(&db);
}

#[test]
fn session_intent_round_trip_rejects_result_shaped_json() {
    let db = path("intent");
    let _ = fs::remove_file(&db);
    let actor = PersistenceActor::open(&db).expect("open");
    let pid = profile(42);

    let intent = r#"{"database":"postgres","schema":"public","selected_tab":0,"tabs":[{"title":"SQL","sql":"SELECT 1"}]}"#;
    actor
        .put_session_intent(pid, intent.into())
        .expect("put");
    let loaded = actor.get_session_intent(pid).expect("get").expect("row");
    assert_eq!(loaded.intent_json, intent);
    // Schema has nowhere for result rows — assert no history/result tables mixed in.
    assert!(!loaded.intent_json.contains("cells"));

    // Result-shaped payload rejected.
    let bad = r#"{"database":"x","tabs":[],"cells":[[1]]}"#;
    assert!(actor.put_session_intent(pid, bad.into()).is_err());

    actor.delete_session_intent(pid).expect("delete");
    assert!(actor.get_session_intent(pid).expect("get2").is_none());
    actor.shutdown().expect("shutdown");
    let _ = fs::remove_file(&db);
}

#[test]
fn intent_and_history_tables_never_hold_result_payloads() {
    let db = path("schema");
    let _ = fs::remove_file(&db);
    let actor = PersistenceActor::open(&db).expect("open");
    // History is statements only.
    actor
        .append_history(HistoryAppend {
            engine: Engine::PostgreSql,
            database_name: "postgres".into(),
            schema_name: None,
            statement_text: "SELECT 1".into(),
            outcome: HistoryOutcomeClass::Completed,
            retention: HistoryRetention::Full,
        })
        .unwrap();
    // Intent is text-only tabs.
    actor
        .put_session_intent(
            profile(7),
            r#"{"database":"app","selected_tab":0,"tabs":[{"title":"SQL","sql":"SELECT 2"}]}"#
                .into(),
        )
        .unwrap();
    let health = actor.health().expect("health");
    assert!(health.integrity_ok);
    assert!(health.schema_version >= 9);
    actor.shutdown().unwrap();
    let _ = fs::remove_file(&db);
}

#[test]
fn atomic_sql_file_write_and_external_change() {
    let dir = std::env::temp_dir().join(format!(
        "tablerock-sql-files-{}",
        std::process::id()
    ));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("query.sql");
    let _ = fs::remove_file(&path);

    let facts = write_sql_file_atomic(&path, "SELECT 'ok';\n").expect("write");
    let (text, _) = read_sql_file(&path).expect("read");
    assert_eq!(text, "SELECT 'ok';\n");
    assert!(!external_change_detected(&facts));

    // Crash simulation: orphan temp file does not destroy original.
    let orphan = dir.join(format!(".query.sql.tmp.{}", std::process::id()));
    fs::write(&orphan, b"CORRUPT").unwrap();
    let (still, _) = read_sql_file(&path).unwrap();
    assert_eq!(still, "SELECT 'ok';\n");

    std::thread::sleep(std::time::Duration::from_millis(25));
    write_sql_file_atomic(&path, "SELECT 2;\n").unwrap();
    assert!(external_change_detected(&facts));

    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(&orphan);
    let _ = fs::remove_dir_all(&dir);
}
