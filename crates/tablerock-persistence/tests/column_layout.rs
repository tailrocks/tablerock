use std::{fs, path::PathBuf};

use tablerock_core::{IdParts, ProfileId};
use tablerock_persistence::{ColumnLayoutKey, PersistenceActor};

fn path(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "tablerock-column-layout-{}-{suffix}.db",
        std::process::id(),
    ))
}

fn profile(low: u64) -> ProfileId {
    ProfileId::from_parts(IdParts::new(1, low).unwrap()).unwrap()
}

#[test]
fn put_get_delete_column_layout() {
    let db = path("crud");
    let _ = fs::remove_file(&db);
    let actor = PersistenceActor::open(&db).expect("open");
    let key = ColumnLayoutKey {
        profile_id: profile(9),
        database: "postgres".into(),
        schema: "public".into(),
        table: "users".into(),
    };
    let json =
        r#"[{"name":"id","visible":true,"width":8},{"name":"name","visible":false,"width":16}]"#;
    actor
        .put_column_layout(key.clone(), json.into())
        .expect("put");
    let loaded = actor
        .get_column_layout(key.clone())
        .expect("get")
        .expect("row");
    assert_eq!(loaded.layout_json, json);
    assert!(actor.health().unwrap().schema_version >= 10);

    // Reject result-shaped payloads.
    assert!(
        actor
            .put_column_layout(key.clone(), r#"{"cells":[]}"#.into())
            .is_err()
    );

    actor.delete_column_layout(key.clone()).expect("delete");
    assert!(actor.get_column_layout(key).expect("get2").is_none());
    actor.shutdown().unwrap();
    let _ = fs::remove_file(&db);
}
