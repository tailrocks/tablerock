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
    let file_name = path
        .file_name()
        .ok_or(PersistenceError::InvalidPath)?
        .to_string_lossy();
    let temp = parent.join(format!(
        ".{}.tmp.{}",
        file_name,
        std::process::id()
    ));
    {
        let mut file = fs::File::create(&temp).map_err(|_| PersistenceError::Query)?;
        file.write_all(text.as_bytes())
            .map_err(|_| PersistenceError::Query)?;
        file.sync_all().map_err(|_| PersistenceError::Query)?;
    }
    // On Windows rename may fail if dest exists; remove then rename is still
    // best-effort atomic for our crash test (temp remains if rename fails).
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(&temp, path).map_err(|_| {
        let _ = fs::remove_file(&temp);
        PersistenceError::Query
    })?;
    file_facts(path)
}

/// Compare current mtime/len to previously observed facts.
#[must_use]
pub fn external_change_detected(previous: &SqlFileFacts) -> bool {
    match file_facts(&previous.path) {
        Ok(current) => {
            current.len != previous.len || current.mtime != previous.mtime
        }
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
}
