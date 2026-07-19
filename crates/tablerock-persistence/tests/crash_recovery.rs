use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use tablerock_persistence::PersistenceActor;

const CRASH_HELPER: &str = "TABLEROCK_PERSISTENCE_CRASH_HELPER";
const CRASH_PATH: &str = "TABLEROCK_PERSISTENCE_CRASH_PATH";

fn crash_database() -> PathBuf {
    std::env::temp_dir().join(format!(
        "tablerock-persistence-crash-{}.db",
        std::process::id()
    ))
}

fn remove_database_files(path: &Path) {
    let _ = fs::remove_file(path);
    for suffix in ["-wal", "-shm"] {
        let mut companion = path.as_os_str().to_os_string();
        companion.push(suffix);
        let _ = fs::remove_file(PathBuf::from(companion));
    }
}

#[test]
fn crash_helper_process() {
    if std::env::var_os(CRASH_HELPER).is_none() {
        return;
    }
    let path = PathBuf::from(std::env::var_os(CRASH_PATH).unwrap());
    let actor = PersistenceActor::open(path).unwrap();
    let health = actor.health().unwrap();
    assert_eq!(health.schema_version, 17);
    assert!(health.foreign_keys_enabled);
    assert!(health.integrity_ok);
    std::process::abort();
}

#[test]
fn abrupt_process_death_reopens_without_drop_or_checkpoint() {
    let path = crash_database();
    remove_database_files(&path);
    let output = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("crash_helper_process")
        .arg("--nocapture")
        .env(CRASH_HELPER, "1")
        .env(CRASH_PATH, &path)
        .output()
        .unwrap();
    assert!(!output.status.success());

    let reopened = PersistenceActor::open(&path).unwrap();
    let health = reopened.health().unwrap();
    assert_eq!(health.schema_version, 17);
    assert!(health.foreign_keys_enabled);
    assert!(health.integrity_ok);
    reopened.shutdown().unwrap();
    remove_database_files(&path);
}
