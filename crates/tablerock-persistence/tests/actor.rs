use std::{
    fs,
    path::{Path, PathBuf},
};

use tablerock_persistence::{PersistenceActor, PersistenceError};

fn temporary_database() -> PathBuf {
    let unique = format!(
        "tablerock-persistence-{}-{}.db",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    );
    std::env::temp_dir().join(unique)
}

fn write_migration_ledger(path: &Path, versions: &[u32]) {
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
            .execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY NOT NULL);")
            .await
            .unwrap();
        for version in versions {
            connection
                .execute(
                    "INSERT INTO schema_migrations(version) VALUES (?1)",
                    (*version,),
                )
                .await
                .unwrap();
        }
    });
}

#[test]
fn actor_opens_migrates_checks_and_reopens_one_local_file() {
    let path = temporary_database();
    let _ = fs::remove_file(&path);
    let actor = PersistenceActor::open(&path).unwrap();
    let health = actor.health().unwrap();
    assert_eq!(health.schema_version, 3);
    assert!(health.foreign_keys_enabled);
    assert!(health.integrity_ok);
    actor.shutdown().unwrap();
    assert!(path.is_file());

    let backup = path.with_extension("backup.db");
    let _ = fs::remove_file(&backup);
    fs::copy(&path, &backup).unwrap();

    let reopened = PersistenceActor::open(&path).unwrap();
    assert_eq!(reopened.health().unwrap(), health);
    reopened.shutdown().unwrap();
    let restored = PersistenceActor::open(&backup).unwrap();
    assert_eq!(restored.health().unwrap(), health);
    restored.shutdown().unwrap();
    fs::remove_file(path).unwrap();
    fs::remove_file(backup).unwrap();
}

#[test]
fn corrupt_files_fail_closed_without_becoming_new_databases() {
    let path = temporary_database().with_extension("corrupt.db");
    fs::write(&path, b"not a database").unwrap();
    assert!(PersistenceActor::open(&path).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"not a database");
    fs::remove_file(&path).unwrap();
    let replacement = PersistenceActor::open(&path).unwrap();
    replacement.shutdown().unwrap();
    fs::remove_file(path).unwrap();
}

#[test]
fn future_and_gapped_migration_ledgers_fail_closed() {
    let future = temporary_database().with_extension("future.db");
    let gap = temporary_database().with_extension("gap.db");
    let _ = fs::remove_file(&future);
    let _ = fs::remove_file(&gap);
    write_migration_ledger(&future, &[1, 2, 99]);
    write_migration_ledger(&gap, &[2]);

    assert!(PersistenceActor::open(&future).is_err());
    assert!(PersistenceActor::open(&gap).is_err());

    fs::remove_file(future).unwrap();
    fs::remove_file(gap).unwrap();
}

#[test]
fn one_normalized_path_has_exactly_one_live_actor() {
    let path = temporary_database().with_extension("ownership.db");
    let _ = fs::remove_file(&path);
    let first = PersistenceActor::open(&path).unwrap();
    let alias = path
        .parent()
        .unwrap()
        .join(".")
        .join(path.file_name().unwrap());

    assert!(matches!(
        PersistenceActor::open(alias),
        Err(PersistenceError::DatabaseBusy)
    ));

    first.shutdown().unwrap();
    let reopened = PersistenceActor::open(&path).unwrap();
    reopened.shutdown().unwrap();
    fs::remove_file(path).unwrap();
}

#[test]
fn restart_rolls_back_an_interrupted_transactional_migration() {
    let path = temporary_database().with_extension("interrupted.db");
    let _ = fs::remove_file(&path);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let database = turso::Builder::new_local(path.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let mut connection = database.connect().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations(\
                    version INTEGER PRIMARY KEY NOT NULL,\
                    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP\
                );\
                INSERT INTO schema_migrations(version) VALUES (1);",
            )
            .await
            .unwrap();
        let transaction = connection.transaction().await.unwrap();
        transaction
            .execute_batch(
                "CREATE TABLE support_facts(\
                    fact_key TEXT PRIMARY KEY NOT NULL,\
                    fact_value TEXT NOT NULL,\
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP\
                );",
            )
            .await
            .unwrap();
        drop(transaction);
    });

    let actor = PersistenceActor::open(&path).unwrap();
    assert_eq!(actor.health().unwrap().schema_version, 3);
    actor.shutdown().unwrap();
    fs::remove_file(path).unwrap();
}
