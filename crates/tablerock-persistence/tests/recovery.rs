use std::{
    fs,
    path::{Path, PathBuf},
};

use tablerock_persistence::{
    BACKUP_FORMAT_VERSION, PersistenceActor, PersistenceError, create_backup, read_backup_manifest,
    restore_backup,
};

fn paths(case: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let root =
        std::env::temp_dir().join(format!("tablerock-recovery-{}-{case}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).unwrap();
    (
        root.join("state.db"),
        root.join("state.backup"),
        root.join("restored.db"),
        root,
    )
}

fn manifest_path(backup: &Path) -> PathBuf {
    PathBuf::from(format!("{}.manifest", backup.display()))
}

#[test]
fn creates_verified_manifest_and_restores_to_independent_file() {
    let (database, backup, restored, root) = paths("roundtrip");
    let actor = PersistenceActor::open(&database).unwrap();
    let expected_health = actor.health().unwrap();
    actor.shutdown().unwrap();

    let manifest = create_backup(&database, &backup).unwrap();
    assert_eq!(manifest.format_version, BACKUP_FORMAT_VERSION);
    assert_eq!(manifest.schema_version, expected_health.schema_version);
    assert_eq!(
        manifest.database_bytes,
        fs::metadata(&backup).unwrap().len()
    );
    assert_eq!(read_backup_manifest(&backup).unwrap(), manifest);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&backup).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(manifest_path(&backup))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    assert_eq!(restore_backup(&backup, &restored).unwrap(), manifest);
    let restored_actor = PersistenceActor::open(&restored).unwrap();
    assert_eq!(restored_actor.health().unwrap(), expected_health);
    restored_actor.shutdown().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&restored).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    assert_eq!(fs::read(&database).unwrap(), fs::read(&restored).unwrap());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn refuses_live_source_and_never_overwrites_backup_or_restore_target() {
    let (database, backup, restored, root) = paths("ownership");
    let actor = PersistenceActor::open(&database).unwrap();
    assert!(matches!(
        create_backup(&database, &backup),
        Err(PersistenceError::DatabaseBusy)
    ));
    actor.shutdown().unwrap();

    create_backup(&database, &backup).unwrap();
    assert!(matches!(
        create_backup(&database, &backup),
        Err(PersistenceError::BackupDestinationExists)
    ));
    fs::write(&restored, b"preserve failed original").unwrap();
    assert!(matches!(
        restore_backup(&backup, &restored),
        Err(PersistenceError::RestoreTargetExists)
    ));
    assert_eq!(fs::read(&restored).unwrap(), b"preserve failed original");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn detects_modified_backup_before_creating_restore_target() {
    let (database, backup, restored, root) = paths("tampered-backup");
    PersistenceActor::open(&database)
        .unwrap()
        .shutdown()
        .unwrap();
    create_backup(&database, &backup).unwrap();
    fs::write(&backup, b"tampered but bounded").unwrap();

    assert!(matches!(
        restore_backup(&backup, &restored),
        Err(PersistenceError::BackupVerification)
    ));
    assert!(!restored.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_unknown_or_malformed_manifest_before_restore() {
    let (database, backup, restored, root) = paths("manifest");
    PersistenceActor::open(&database)
        .unwrap()
        .shutdown()
        .unwrap();
    create_backup(&database, &backup).unwrap();
    fs::write(
        manifest_path(&backup),
        "format=99\nschema=6\nbytes=1\nsha256=00\n",
    )
    .unwrap();

    assert!(matches!(
        restore_backup(&backup, &restored),
        Err(PersistenceError::BackupManifest)
    ));
    assert!(!restored.exists());
    fs::remove_dir_all(root).unwrap();
}
