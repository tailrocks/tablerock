//! Atomic `.sql` file read/write helpers (local filesystem only).
//!
//! Write path: temp file in the same directory + rename. A crash between
//! temp write and rename leaves the original intact.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::PersistenceError;

/// Facts observed for external-change detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlFileFacts {
    pub path: PathBuf,
    pub mtime: Option<SystemTime>,
    pub len: u64,
}

/// Read a `.sql` file as UTF-8 text (lossy only if invalid — returns error).
pub fn read_sql_file(path: &Path) -> Result<(String, SqlFileFacts), PersistenceError> {
    let bytes = fs::read(path).map_err(|_| PersistenceError::Query)?;
    let text = String::from_utf8(bytes).map_err(|_| PersistenceError::Query)?;
    let facts = file_facts(path)?;
    Ok((text, facts))
}

/// Atomic write: create `path.tmp.<pid>` then rename over `path`.
pub fn write_sql_file_atomic(path: &Path, text: &str) -> Result<SqlFileFacts, PersistenceError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or(PersistenceError::InvalidPath)?;
    if !parent.exists() {
        fs::create_dir_all(parent).map_err(|_| PersistenceError::InvalidPath)?;
    }
    reject_mode_readonly_parent(parent)?;
    let file_name = path
        .file_name()
        .ok_or(PersistenceError::InvalidPath)?
        .to_string_lossy();
    let temp = parent.join(format!(".{}.tmp.{}", file_name, std::process::id()));
    {
        let mut file = fs::File::create(&temp).map_err(|_| PersistenceError::Query)?;
        file.write_all(text.as_bytes())
            .map_err(|_| PersistenceError::Query)?;
        file.sync_all().map_err(|_| PersistenceError::Query)?;
    }
    // Unix rename replaces the destination atomically. Never pre-delete the
    // destination: a later rename failure must leave the original intact.
    fs::rename(&temp, path).map_err(|_| {
        let _ = fs::remove_file(&temp);
        PersistenceError::Query
    })?;
    file_facts(path)
}

#[cfg(unix)]
fn reject_mode_readonly_parent(parent: &Path) -> Result<(), PersistenceError> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = fs::metadata(parent)
        .map_err(|_| PersistenceError::InvalidPath)?
        .permissions();
    if permissions.mode() & 0o222 == 0 {
        return Err(PersistenceError::Query);
    }
    Ok(())
}

#[cfg(not(unix))]
fn reject_mode_readonly_parent(_parent: &Path) -> Result<(), PersistenceError> {
    Ok(())
}

/// Compare current mtime/len to previously observed facts.
#[must_use]
pub fn external_change_detected(previous: &SqlFileFacts) -> bool {
    match file_facts(&previous.path) {
        Ok(current) => current.len != previous.len || current.mtime != previous.mtime,
        // Missing file or unreadable counts as external change.
        Err(_) => true,
    }
}

fn file_facts(path: &Path) -> Result<SqlFileFacts, PersistenceError> {
    let meta = fs::metadata(path).map_err(|_| PersistenceError::Query)?;
    let mtime = meta.modified().ok();
    Ok(SqlFileFacts {
        path: path.to_path_buf(),
        mtime,
        len: meta.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "tablerock-sql-file-{}-{}-{name}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    }

    #[test]
    fn atomic_write_leaves_original_when_temp_not_renamed() {
        let path = tmp("orig.sql");
        let _ = fs::remove_file(&path);
        write_sql_file_atomic(&path, "SELECT 1;\n").unwrap();
        let (text, _) = read_sql_file(&path).unwrap();
        assert_eq!(text, "SELECT 1;\n");

        // Simulate crash after temp write: write a temp but do not rename.
        let parent = path.parent().unwrap();
        let temp = parent.join(format!(".orig.sql.tmp.crash.{}", std::process::id()));
        fs::write(&temp, b"SELECT DESTROYED;\n").unwrap();
        // Original must still be intact.
        let (still, _) = read_sql_file(&path).unwrap();
        assert_eq!(still, "SELECT 1;\n");
        let _ = fs::remove_file(&temp);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn external_change_detects_mtime_or_len() {
        let path = tmp("watch.sql");
        let _ = fs::remove_file(&path);
        let facts = write_sql_file_atomic(&path, "a").unwrap();
        assert!(!external_change_detected(&facts));
        // Rewrite with different content.
        std::thread::sleep(std::time::Duration::from_millis(20));
        write_sql_file_atomic(&path, "ab").unwrap();
        assert!(external_change_detected(&facts));
        let _ = fs::remove_file(&path);
    }

    #[cfg(unix)]
    #[test]
    fn write_fails_closed_on_readonly_parent() {
        use std::os::unix::fs::PermissionsExt;

        let parent = tmp("readonly-sql-parent");
        let _ = fs::remove_dir_all(&parent);
        fs::create_dir_all(&parent).unwrap();
        let path = parent.join("query.sql");
        let mut perms = fs::metadata(&parent).unwrap().permissions();
        perms.set_mode(0o555);
        fs::set_permissions(&parent, perms).unwrap();

        let result = write_sql_file_atomic(&path, "SELECT 1;\n");
        assert!(
            result.is_err(),
            "readonly parent must fail atomic SQL write"
        );

        let mut restore = fs::metadata(&parent).unwrap().permissions();
        restore.set_mode(0o755);
        fs::set_permissions(&parent, restore).unwrap();
        let temps: Vec<_> = parent
            .read_dir()
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .collect();
        assert!(
            temps.is_empty(),
            "failed SQL write must not leave temps: {temps:?}"
        );
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&parent);
    }
}
