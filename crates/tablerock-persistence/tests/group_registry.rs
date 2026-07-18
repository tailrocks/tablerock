use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use tablerock_persistence::PersistenceActor;

fn path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "tablerock-group-registry-{}-{}.db",
        std::process::id(),
        format!(
            "{:?}-{}",
            std::thread::current().id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
        .replace(['(', ')'], "")
    ))
}

#[test]
fn empty_groups_survive_rename_and_delete_without_profiles() {
    let path = path();
    let _ = fs::remove_file(&path);
    let actor = PersistenceActor::open(&path).unwrap();

    actor.create_group("Operations").unwrap();
    assert_eq!(actor.list_groups().unwrap(), ["Operations"]);
    assert_eq!(actor.rename_group("Operations", "Analytics").unwrap(), 0);
    assert_eq!(actor.list_groups().unwrap(), ["Analytics"]);
    assert_eq!(actor.delete_group("Analytics").unwrap(), 0);
    assert!(actor.list_groups().unwrap().is_empty());

    actor.shutdown().unwrap();
    fs::remove_file(path).unwrap();
}

#[test]
fn migration_backfills_groups_from_existing_profiles() {
    let path = path();
    let actor = PersistenceActor::open(&path).unwrap();
    actor.shutdown().unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let database = turso::Builder::new_local(path.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let connection = database.connect().unwrap();
        connection
            .execute_batch(
                "DROP TABLE saved_profile_groups;
                 DELETE FROM schema_migrations WHERE version = 14;
                 INSERT INTO saved_profiles(
                    profile_id, aggregate_schema, connection_schema, property_schema,
                    revision, engine, name, tls_policy, safety_mode, connect_timeout_ms,
                    operation_timeout_ms, max_result_rows, max_result_bytes, group_name,
                    favorite, saved_order, reconnect, restore_last_context,
                    preferred_page_rows, environment_kind, environment_label, ssh_use_agent
                 ) VALUES (
                    X'00000000000000000000000000000001', 1, 1, 1,
                    X'0000000000000000', 1, 'Existing', 1, 1, 1000,
                    1000, 100, 1024, 'Legacy', 0, 0, 1, 1, 100,
                    NULL, NULL, 0
                 );",
            )
            .await
            .unwrap();
    });

    let reopened = PersistenceActor::open(&path).unwrap();
    assert_eq!(reopened.health().unwrap().schema_version, 14);
    assert_eq!(reopened.list_groups().unwrap(), ["Legacy"]);
    assert_eq!(reopened.rename_group("Legacy", "Modern").unwrap(), 1);
    assert_eq!(reopened.delete_group("Modern").unwrap(), 1);
    reopened.shutdown().unwrap();
    runtime.block_on(async {
        let database = turso::Builder::new_local(path.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let connection = database.connect().unwrap();
        let mut rows = connection
            .query("SELECT revision, group_name FROM saved_profiles", ())
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        let revision = row.get::<Vec<u8>>(0).unwrap();
        assert_eq!(revision, 2_u64.to_be_bytes());
        assert!(row.get::<Option<String>>(1).unwrap().is_none());
    });
    fs::remove_file(path).unwrap();
}
