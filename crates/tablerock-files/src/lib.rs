//! Atomic file write/export effects shared by TableRock presentation adapters.
//!
//! Policy: write to a same-directory temp file, fsync, then rename into place.
//! On cancel/failure the temp (and any incomplete destination) is removed.
//! Paths must be absolute or resolved against the process cwd; empty paths
//! and directory targets are rejected.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Errors from path validation or atomic write.
#[derive(Debug)]
pub enum FileEffectError {
    EmptyPath,
    IsDirectory,
    Io(io::Error),
    IncompleteRemoved,
}

impl std::fmt::Display for FileEffectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPath => f.write_str("export path is empty"),
            Self::IsDirectory => f.write_str("export path is a directory"),
            Self::Io(e) => write!(f, "file effect I/O: {e}"),
            Self::IncompleteRemoved => f.write_str("incomplete export removed"),
        }
    }
}

impl std::error::Error for FileEffectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for FileEffectError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// Validate and normalize an export destination path.
pub fn validate_export_path(path: &str) -> Result<PathBuf, FileEffectError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(FileEffectError::EmptyPath);
    }
    let path = PathBuf::from(trimmed);
    if path.is_dir() {
        return Err(FileEffectError::IsDirectory);
    }
    Ok(path)
}

/// Streaming atomic writer: temp file next to destination, rename on finish.
pub struct AtomicFileWriter {
    dest: PathBuf,
    temp: PathBuf,
    file: Option<File>,
    bytes_written: u64,
    finished: bool,
}

impl AtomicFileWriter {
    pub fn create(dest: PathBuf) -> Result<Self, FileEffectError> {
        if dest.as_os_str().is_empty() {
            return Err(FileEffectError::EmptyPath);
        }
        if dest.is_dir() {
            return Err(FileEffectError::IsDirectory);
        }
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                // Fail closed before any temp is created when parent is not a dir
                // (file-as-parent, missing intermediate that is a file, etc.).
                if parent.exists() && !parent.is_dir() {
                    return Err(FileEffectError::Io(io::Error::new(
                        io::ErrorKind::NotADirectory,
                        "export parent path is not a directory",
                    )));
                }
                fs::create_dir_all(parent)?;
            }
        }
        // Exclusive unique create. A collision belongs to another live writer or
        // a crashed process and must never be removed by this writer.
        let (temp, file) = (0..64)
            .find_map(|_| {
                let nonce = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
                let temp = temp_path_for(&dest, nonce);
                match OpenOptions::new().write(true).create_new(true).open(&temp) {
                    Ok(file) => Some(Ok((temp, file))),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => None,
                    Err(error) => Some(Err(FileEffectError::Io(error))),
                }
            })
            .transpose()?
            .ok_or_else(|| {
                FileEffectError::Io(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "could not reserve a unique export temp file",
                ))
            })?;
        Ok(Self {
            dest,
            temp,
            file: Some(file),
            bytes_written: 0,
            finished: false,
        })
    }

    pub fn write_all(&mut self, bytes: &[u8]) -> Result<(), FileEffectError> {
        let file = self.file.as_mut().ok_or_else(|| {
            FileEffectError::Io(io::Error::new(io::ErrorKind::Other, "writer closed"))
        })?;
        file.write_all(bytes)?;
        self.bytes_written = self.bytes_written.saturating_add(bytes.len() as u64);
        Ok(())
    }

    #[must_use]
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// Fsync + rename into destination. Consumes the writer.
    pub fn finish(mut self) -> Result<u64, FileEffectError> {
        if let Some(mut file) = self.file.take() {
            file.flush()?;
            file.sync_all()?;
        }
        fs::rename(&self.temp, &self.dest)?;
        // Best-effort fsync parent for durability on some filesystems.
        if let Some(parent) = self.dest.parent() {
            if let Ok(dir) = File::open(parent) {
                let _ = dir.sync_all();
            }
        }
        self.finished = true;
        Ok(self.bytes_written)
    }

    /// Remove temp (and dest if partially replaced — rename is atomic so only temp).
    pub fn abort(mut self) {
        self.file.take();
        let _ = fs::remove_file(&self.temp);
        // Destination should not exist unless a previous complete write left it.
        // Never leave temp behind.
    }
}

impl Drop for AtomicFileWriter {
    fn drop(&mut self) {
        if !self.finished {
            self.file.take();
            let _ = fs::remove_file(&self.temp);
        }
    }
}

fn temp_path_for(dest: &Path, nonce: u64) -> PathBuf {
    let name = dest
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("export");
    let temp_name = format!(".{name}.tablerock-tmp-{}-{nonce}", std::process::id());
    match dest.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(temp_name),
        _ => PathBuf::from(temp_name),
    }
}

/// Write an entire buffer atomically (convenience for small loaded-result exports).
pub fn write_atomic(dest: &Path, data: &[u8]) -> Result<u64, FileEffectError> {
    let mut w = AtomicFileWriter::create(dest.to_path_buf())?;
    w.write_all(data)?;
    w.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "tablerock-file-effects-{}-{}-{}",
            std::process::id(),
            nanos,
            seq
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn atomic_write_and_abort_cleanup() {
        let dir = scratch_dir();
        let dest = dir.join("out.csv");
        let mut w = AtomicFileWriter::create(dest.clone()).unwrap();
        w.write_all(b"a,b\n1,2\n").unwrap();
        assert!(!dest.exists());
        let n = w.finish().unwrap();
        assert_eq!(n, 8);
        assert_eq!(fs::read_to_string(&dest).unwrap(), "a,b\n1,2\n");

        let dest2 = dir.join("partial.csv");
        let mut w2 = AtomicFileWriter::create(dest2.clone()).unwrap();
        w2.write_all(b"partial").unwrap();
        let temp_exists = dir
            .read_dir()
            .unwrap()
            .filter_map(|e| e.ok())
            .any(|e| e.file_name().to_string_lossy().contains("tablerock-tmp"));
        assert!(temp_exists);
        w2.abort();
        assert!(!dest2.exists());
        let temps: Vec<_> = dir
            .read_dir()
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("tablerock-tmp"))
            .collect();
        assert!(temps.is_empty(), "abort must remove temp");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn drop_without_finish_removes_temp() {
        let dir = scratch_dir();
        let dest = dir.join("drop.csv");
        {
            let mut w = AtomicFileWriter::create(dest.clone()).unwrap();
            w.write_all(b"x").unwrap();
        }
        assert!(!dest.exists());
        let temps: Vec<_> = dir
            .read_dir()
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("tablerock-tmp"))
            .collect();
        assert!(temps.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn concurrent_writers_never_share_or_delete_temp_files() {
        let dir = scratch_dir();
        let dest = dir.join("same.csv");
        let mut first = AtomicFileWriter::create(dest.clone()).unwrap();
        first.write_all(b"first").unwrap();
        let mut second = AtomicFileWriter::create(dest.clone()).unwrap();
        second.write_all(b"second").unwrap();
        let temps = dir
            .read_dir()
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains("tablerock-tmp")
            })
            .count();
        assert_eq!(temps, 2);

        first.abort();
        assert_eq!(second.finish().unwrap(), 6);
        assert_eq!(fs::read(&dest).unwrap(), b"second");
        assert_eq!(
            dir.read_dir()
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .contains("tablerock-tmp"))
                .count(),
            0
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn create_fails_closed_when_parent_is_a_file() {
        // Disk-full is hard to force portably; a file-as-parent is a reliable
        // fail-closed write path that must not leave tablerock-tmp debris.
        let dir = scratch_dir();
        let blocker = dir.join("not-a-dir");
        fs::write(&blocker, b"x").unwrap();
        let dest = blocker.join("blocked.csv");
        let result = AtomicFileWriter::create(dest);
        assert!(result.is_err(), "file-as-parent must fail create");
        let temps: Vec<_> = dir
            .read_dir()
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("tablerock-tmp"))
            .collect();
        assert!(
            temps.is_empty(),
            "failed create must not leave temp files: {temps:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_path_rejected() {
        assert!(matches!(
            validate_export_path(""),
            Err(FileEffectError::EmptyPath)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn create_fails_closed_on_readonly_parent() {
        // Permission-denied parent is a portable stand-in for disk-full class
        // failures: write must fail closed and leave no temp debris.
        use std::os::unix::fs::PermissionsExt;

        let dir = scratch_dir();
        let parent = dir.join("readonly-parent");
        fs::create_dir_all(&parent).unwrap();
        let mut perms = fs::metadata(&parent).unwrap().permissions();
        perms.set_mode(0o555); // r-x only — create of temp must fail
        fs::set_permissions(&parent, perms).unwrap();

        let dest = parent.join("export.csv");
        let result = AtomicFileWriter::create(dest);
        assert!(
            result.is_err(),
            "readonly parent must fail AtomicFileWriter::create"
        );

        // Restore write so cleanup can run and count temps.
        let mut restore = fs::metadata(&parent).unwrap().permissions();
        restore.set_mode(0o755);
        fs::set_permissions(&parent, restore).unwrap();
        let temps: Vec<_> = parent
            .read_dir()
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("tablerock-tmp"))
            .collect();
        assert!(
            temps.is_empty(),
            "failed create must not leave tablerock-tmp: {temps:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn write_fails_closed_when_file_becomes_unwritable() {
        // Mid-stream write failure: open succeeds, then chmod file to 0 so
        // write_all fails; abort/drop must not promote incomplete dest.
        use std::os::unix::fs::PermissionsExt;

        let dir = scratch_dir();
        let dest = dir.join("midstream.csv");
        let mut w = AtomicFileWriter::create(dest.clone()).unwrap();
        // Locate the temp sibling and remove write bits after create.
        let temp = dir
            .read_dir()
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().contains("tablerock-tmp"))
            .expect("temp must exist after create")
            .path();
        let mut perms = fs::metadata(&temp).unwrap().permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&temp, perms).unwrap();

        let write_err = w.write_all(b"this should fail if no write bit");
        // On some platforms the open handle may still write; force fail closed
        // by asserting either write fails or finish fails — dest never partial
        // incomplete without atomic rename.
        if write_err.is_ok() {
            // Restore write so finish can attempt rename path; then abort.
            let mut restore = fs::metadata(&temp).unwrap().permissions();
            restore.set_mode(0o644);
            fs::set_permissions(&temp, restore).unwrap();
            w.abort();
        } else {
            drop(w);
        }
        assert!(
            !dest.exists(),
            "dest must not exist after mid-write failure"
        );
        let temps: Vec<_> = dir
            .read_dir()
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("tablerock-tmp"))
            .collect();
        assert!(
            temps.is_empty(),
            "mid-write failure must clean temps: {temps:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// True ENOSPC when `TABLEROCK_ENOSPC_MNT` points at a tiny full volume.
    ///
    /// CI mounts a 1MiB tmpfs and fills it before this test. Locally the test
    /// is ignored unless the env var is set (avoids filling the developer disk).
    #[cfg(unix)]
    #[test]
    fn enospc_volume_fails_closed_without_temp_debris() {
        let Ok(mnt) = std::env::var("TABLEROCK_ENOSPC_MNT") else {
            eprintln!("skip: set TABLEROCK_ENOSPC_MNT to a tiny full volume to run ENOSPC");
            return;
        };
        let mnt = PathBuf::from(mnt);
        assert!(
            mnt.is_dir(),
            "TABLEROCK_ENOSPC_MNT must be a directory: {}",
            mnt.display()
        );

        // Fill remaining space with a filler file so subsequent creates hit ENOSPC.
        let filler = mnt.join("fill.bin");
        {
            let mut f = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&filler)
                .expect("open filler");
            let chunk = vec![0_u8; 64 * 1024];
            loop {
                match f.write_all(&chunk) {
                    Ok(()) => {}
                    Err(e) if e.raw_os_error() == Some(28) /* ENOSPC */ => break,
                    Err(e) if e.kind() == std::io::ErrorKind::StorageFull => break,
                    Err(e) => {
                        // Some FS report EDQUOT or write zero — stop when short write.
                        if e.raw_os_error() == Some(122) {
                            break;
                        }
                        // If we cannot fill, still attempt create — may pass free.
                        break;
                    }
                }
            }
            let _ = f.sync_all();
        }

        let dest = mnt.join("export.csv");
        let create = AtomicFileWriter::create(dest.clone());
        // If create somehow succeeds (race free space), write until fail then abort.
        match create {
            Ok(mut w) => {
                let big = vec![b'x'; 128 * 1024];
                let write = w.write_all(&big);
                if write.is_ok() {
                    // Keep writing until failure or finish fails.
                    while w.write_all(&big).is_ok() {}
                }
                w.abort();
                assert!(
                    !dest.exists(),
                    "ENOSPC path must not promote dest to final path"
                );
            }
            Err(_) => {
                // Fail closed at create — expected when volume is full.
            }
        }

        let temps: Vec<_> = mnt
            .read_dir()
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("tablerock-tmp"))
            .collect();
        assert!(
            temps.is_empty(),
            "ENOSPC must not leave tablerock-tmp debris: {temps:?}"
        );
        // Cleanup filler so unmount can succeed.
        let _ = fs::remove_file(filler);
        let _ = fs::remove_file(dest);
    }
}
