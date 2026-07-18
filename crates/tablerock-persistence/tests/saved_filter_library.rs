use std::{fs, path::PathBuf};

use tablerock_core::{IdParts, ProfileId};
use tablerock_persistence::PersistenceActor;

fn path(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "tablerock-saved-filter-{}-{suffix}.db",
        std::process::id(),
    ))
}

fn profile(low: u64) -> ProfileId {
    ProfileId::from_parts(IdParts::new(1, low).unwrap()).unwrap()
}

#[test]
fn put_get_delete_saved_filter_library() {
    let db = path("crud");
    let _ = fs::remove_file(&db);
    let actor = PersistenceActor::open(&db).expect("open");
    assert!(actor.health().unwrap().schema_version >= 13);

    let id = profile(21);
    let json = r#"[{"name":"active","schema":"public","table":"users","raw_where":null,"filters":[{"column":"status","operator":"eq","value":"active"}]}]"#;
    actor
        .put_saved_filter_library(id, json.into())
        .expect("put");
    let loaded = actor
        .get_saved_filter_library(id)
        .expect("get")
        .expect("row");
    assert_eq!(loaded.library_json, json);
    assert_eq!(loaded.profile_id, id);

    // Upsert replaces.
    let json2 = r#"[]"#;
    actor
        .put_saved_filter_library(id, json2.into())
        .expect("put2");
    assert_eq!(
        actor
            .get_saved_filter_library(id)
            .expect("get2")
            .expect("row2")
            .library_json,
        json2
    );

    // Reject result-shaped / secret-shaped payloads.
    assert!(
        actor
            .put_saved_filter_library(id, r#"{"cells":[]}"#.into())
            .is_err()
    );
    assert!(
        actor
            .put_saved_filter_library(id, r#"[{"password":"x"}]"#.into())
            .is_err()
    );
    assert!(
        actor
            .put_saved_filter_library(id, "not-json".into())
            .is_err()
    );

    actor.delete_saved_filter_library(id).expect("delete");
    assert!(actor.get_saved_filter_library(id).expect("get3").is_none());
    actor.shutdown().unwrap();
    let _ = fs::remove_file(&db);
}
