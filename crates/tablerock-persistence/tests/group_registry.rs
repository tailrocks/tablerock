use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use tablerock_core::{IdParts, ProfileId, Revision};
use tablerock_persistence::{PersistenceActor, ProfileOrderUpdate};

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
                "DROP TABLE native_window_session_intent;
                 DROP TABLE saved_profile_groups;
                 DELETE FROM schema_migrations WHERE version >= 14;
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
                 ), (
                    X'00000000000000000000000000000002', 1, 1, 1,
                    X'0000000000000000', 1, 'Second', 1, 1, 1000,
                    1000, 100, 1024, 'Legacy', 0, 1, 1, 1, 100,
                    NULL, NULL, 0
                 ), (
                    X'00000000000000000000000000000003', 1, 1, 1,
                    X'0000000000000000', 1, 'Third', 1, 1, 1000,
                    1000, 100, 1024, 'Legacy', 0, 2, 1, 1, 100,
                    NULL, NULL, 0
                 );",
            )
            .await
            .unwrap();
    });

    let reopened = PersistenceActor::open(&path).unwrap();
    assert_eq!(reopened.health().unwrap().schema_version, 18);
    assert_eq!(reopened.list_groups().unwrap(), ["Legacy"]);
    assert!(!reopened.list_group_settings().unwrap()[0].alphabetical);
    reopened.set_group_alphabetical("Legacy", true).unwrap();
    assert!(reopened.list_group_settings().unwrap()[0].alphabetical);
    let profile = |low| ProfileId::from_parts(IdParts::new(0, low).unwrap()).unwrap();
    reopened
        .set_profile_favorite(profile(1), Revision::INITIAL, true)
        .unwrap();
    assert_eq!(
        reopened
            .set_profile_favorite(profile(1), Revision::INITIAL, false)
            .unwrap_err(),
        tablerock_persistence::PersistenceError::ProfileStaleRevision
    );
    let order = vec![
        ProfileOrderUpdate {
            id: profile(3),
            expected_revision: Revision::INITIAL,
        },
        ProfileOrderUpdate {
            id: profile(1),
            expected_revision: Revision::from_wire_u64(1),
        },
        ProfileOrderUpdate {
            id: profile(2),
            expected_revision: Revision::INITIAL,
        },
    ];
    reopened
        .reorder_profiles(Some("Legacy"), order.clone())
        .unwrap();
    assert_eq!(
        reopened
            .reorder_profiles(Some("Legacy"), order)
            .unwrap_err(),
        tablerock_persistence::PersistenceError::ProfileStaleRevision
    );
    assert_eq!(reopened.rename_group("Legacy", "Modern").unwrap(), 3);
    assert_eq!(reopened.delete_group("Modern").unwrap(), 3);
    reopened.shutdown().unwrap();
    runtime.block_on(async {
        let database = turso::Builder::new_local(path.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let connection = database.connect().unwrap();
        let mut rows = connection
            .query(
                "SELECT revision, group_name, favorite, saved_order \
                 FROM saved_profiles ORDER BY profile_id",
                (),
            )
            .await
            .unwrap();
        let first = rows.next().await.unwrap().unwrap();
        assert_eq!(first.get::<Vec<u8>>(0).unwrap(), 4_u64.to_be_bytes());
        assert!(first.get::<Option<String>>(1).unwrap().is_none());
        assert_eq!(first.get::<u32>(2).unwrap(), 1);
        assert_eq!(first.get::<u32>(3).unwrap(), 1);
        let second = rows.next().await.unwrap().unwrap();
        assert_eq!(second.get::<Vec<u8>>(0).unwrap(), 3_u64.to_be_bytes());
        assert_eq!(second.get::<u32>(3).unwrap(), 2);
        let third = rows.next().await.unwrap().unwrap();
        assert_eq!(third.get::<Vec<u8>>(0).unwrap(), 3_u64.to_be_bytes());
        assert_eq!(third.get::<u32>(3).unwrap(), 0);
    });
    fs::remove_file(path).unwrap();
}
