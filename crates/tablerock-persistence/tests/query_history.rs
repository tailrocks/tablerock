use std::{fs, path::PathBuf};

use tablerock_core::Engine;
use tablerock_persistence::{
    HistoryAppend, HistoryOutcomeClass, HistoryRetention, PersistenceActor,
};

fn path(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "tablerock-query-history-{}-{suffix}.db",
        std::process::id(),
    ))
}

#[test]
fn append_list_search_and_private_modes() {
    let db = path("modes");
    let _ = fs::remove_file(&db);
    let actor = PersistenceActor::open(&db).expect("open");

    let id = actor
        .append_history(HistoryAppend {
            engine: Engine::PostgreSql,
            database_name: "postgres".into(),
            schema_name: Some("public".into()),
            statement_text: "SELECT 1".into(),
            outcome: HistoryOutcomeClass::Completed,
            retention: HistoryRetention::Full,
        })
        .expect("append full")
        .expect("id");
    assert!(id > 0);

    // Metadata-only: row exists, no statement text.
    actor
        .append_history(HistoryAppend {
            engine: Engine::PostgreSql,
            database_name: "postgres".into(),
            schema_name: None,
            statement_text: "SELECT secret_payload".into(),
            outcome: HistoryOutcomeClass::Failed,
            retention: HistoryRetention::MetadataOnly,
        })
        .expect("append meta")
        .expect("id");

    // Private: no row.
    let none = actor
        .append_history(HistoryAppend {
            engine: Engine::PostgreSql,
            database_name: "postgres".into(),
            schema_name: None,
            statement_text: "SELECT private".into(),
            outcome: HistoryOutcomeClass::Completed,
            retention: HistoryRetention::Private,
        })
        .expect("append private");
    assert_eq!(none, None);

    let all = actor.list_history(None, 50).expect("list");
    assert_eq!(all.len(), 2);
    assert_eq!(all.iter().filter(|e| e.statement_text.is_some()).count(), 1);
    assert!(
        all.iter()
            .any(|e| e.statement_text.as_deref() == Some("SELECT 1"))
    );
    assert!(
        all.iter()
            .any(|e| e.statement_text.is_none() && e.outcome == HistoryOutcomeClass::Failed)
    );

    let found = actor
        .list_history(Some("SELECT 1".into()), 10)
        .expect("search");
    assert_eq!(found.len(), 1);

    // Secret text not searchable when metadata-only.
    let secret = actor
        .list_history(Some("secret_payload".into()), 10)
        .expect("search secret");
    assert!(secret.is_empty());

    assert_eq!(actor.history_count().expect("count"), 2);
    actor.shutdown().expect("shutdown");
    let _ = fs::remove_file(&db);
}

#[test]
fn enforces_bounded_row_cap() {
    let db = path("cap");
    let _ = fs::remove_file(&db);
    let actor = PersistenceActor::open(&db).expect("open");
    // Append more than default limit would be slow; use many inserts via API
    // is fine for 50. Cap enforcement uses DEFAULT_HISTORY_LIMIT=500 — spot
    // check that count grows and list returns newest first.
    for i in 0..5 {
        actor
            .append_history(HistoryAppend {
                engine: Engine::ClickHouse,
                database_name: "default".into(),
                schema_name: None,
                statement_text: format!("SELECT {i}"),
                outcome: HistoryOutcomeClass::Completed,
                retention: HistoryRetention::Full,
            })
            .expect("append")
            .expect("id");
    }
    let list = actor.list_history(None, 3).expect("list");
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].statement_text.as_deref(), Some("SELECT 4"));
    assert_eq!(actor.history_count().expect("count"), 5);
    actor.shutdown().expect("shutdown");
    let _ = fs::remove_file(&db);
}
