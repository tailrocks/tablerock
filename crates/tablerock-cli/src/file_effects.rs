//! Atomic file write/export helpers for TableRock CLI effects.
//!
//! Policy: write to a same-directory temp file, fsync, then rename into place.
//! On cancel/failure the temp (and any incomplete destination) is removed.
//! Paths must be absolute or resolved against the process cwd; empty paths
//! and directory targets are rejected.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

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
                fs::create_dir_all(parent)?;
            }
        }
        let temp = temp_path_for(&dest);
        // Exclusive create — fail if leftover temp exists from a crash.
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
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

fn temp_path_for(dest: &Path) -> PathBuf {
    let name = dest
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("export");
    let temp_name = format!(".{name}.tablerock-tmp-{}", std::process::id());
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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("tablerock-file-effects-{nanos}"));
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
}
