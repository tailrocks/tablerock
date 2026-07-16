use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::{
    OPERATION_TIMEOUT, PathLease, PersistenceError, checkpoint, normalize_database_path,
    open_database, read_health, tokio_runtime,
};

/// Current version of TableRock's offline persistence-backup manifest.
pub const BACKUP_FORMAT_VERSION: u32 = 1;
/// Largest database file accepted by the offline backup workflow.
pub const MAX_BACKUP_BYTES: u64 = 512 * 1024 * 1024;
const COPY_BUFFER_BYTES: usize = 64 * 1024;
const MAX_MANIFEST_BYTES: u64 = 512;

/// Path-free metadata used to authenticate and validate an offline backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackupManifest {
    /// Backup-manifest encoding version.
    pub format_version: u32,
    /// Exact persistence schema version observed before copying.
    pub schema_version: u32,
    /// Exact database-file length covered by `sha256`.
    pub database_bytes: u64,
    /// SHA-256 digest of the complete checkpointed database file.
    pub sha256: [u8; 32],
}

/// Creates a bounded, checkpointed backup and atomic sidecar manifest.
///
/// Both destination paths must be absent and no actor may own the source.
pub fn create_backup(
    database_path: impl AsRef<Path>,
    backup_path: impl AsRef<Path>,
) -> Result<BackupManifest, PersistenceError> {
    let database_path = normalize_database_path(database_path.as_ref())?;
    let backup_path = normalize_database_path(backup_path.as_ref())?;
    let manifest_path = manifest_path(&backup_path);
    if database_path == backup_path || !database_path.is_file() {
        return Err(PersistenceError::BackupSource);
    }
    if backup_path.exists() || manifest_path.exists() {
        return Err(PersistenceError::BackupDestinationExists);
    }

    let _database_lease = PathLease::acquire(database_path.clone())?;
    let _backup_lease = PathLease::acquire(backup_path.clone())?;
    let health = offline_health(&database_path, true)?;
    if !health.integrity_ok {
        return Err(PersistenceError::BackupVerification);
    }

    let (database_bytes, sha256) = copy_atomic(&database_path, &backup_path)?;
    let backup_health = offline_health(&backup_path, false).inspect_err(|_| {
        let _ = fs::remove_file(&backup_path);
    })?;
    if backup_health != health {
        let _ = fs::remove_file(&backup_path);
        return Err(PersistenceError::BackupVerification);
    }

    let manifest = BackupManifest {
        format_version: BACKUP_FORMAT_VERSION,
        schema_version: health.schema_version,
        database_bytes,
        sha256,
    };
    if write_manifest_atomic(&manifest_path, manifest).is_err() {
        let _ = fs::remove_file(&backup_path);
        return Err(PersistenceError::BackupIo);
    }
    sync_parent(&backup_path)?;
    Ok(manifest)
}

/// Reads and validates the bounded sidecar manifest for `backup_path`.
pub fn read_backup_manifest(
    backup_path: impl AsRef<Path>,
) -> Result<BackupManifest, PersistenceError> {
    let backup_path = normalize_database_path(backup_path.as_ref())?;
    read_manifest(&manifest_path(&backup_path))
}

/// Verifies a backup and restores it to a new, independently validated file.
///
/// The destination must be absent. This function never replaces or removes an
/// existing database.
pub fn restore_backup(
    backup_path: impl AsRef<Path>,
    restore_path: impl AsRef<Path>,
) -> Result<BackupManifest, PersistenceError> {
    let backup_path = normalize_database_path(backup_path.as_ref())?;
    let restore_path = normalize_database_path(restore_path.as_ref())?;
    if !backup_path.is_file() || backup_path == restore_path {
        return Err(PersistenceError::BackupSource);
    }
    if restore_path.exists() {
        return Err(PersistenceError::RestoreTargetExists);
    }

    let _backup_lease = PathLease::acquire(backup_path.clone())?;
    let _restore_lease = PathLease::acquire(restore_path.clone())?;
    let manifest = read_manifest(&manifest_path(&backup_path))?;
    verify_file(&backup_path, manifest)?;
    let backup_health = offline_health(&backup_path, false)?;
    if !backup_health.integrity_ok || backup_health.schema_version != manifest.schema_version {
        return Err(PersistenceError::BackupVerification);
    }

    let (database_bytes, sha256) = copy_atomic(&backup_path, &restore_path)?;
    if database_bytes != manifest.database_bytes || sha256 != manifest.sha256 {
        let _ = fs::remove_file(&restore_path);
        return Err(PersistenceError::BackupVerification);
    }
    let restored_health = offline_health(&restore_path, false).inspect_err(|_| {
        let _ = fs::remove_file(&restore_path);
    })?;
    if restored_health != backup_health {
        let _ = fs::remove_file(&restore_path);
        return Err(PersistenceError::BackupVerification);
    }
    sync_parent(&restore_path)?;
    Ok(manifest)
}

fn offline_health(
    path: &Path,
    checkpoint_before_read: bool,
) -> Result<crate::PersistenceHealth, PersistenceError> {
    let runtime = tokio_runtime()?;
    runtime.block_on(async {
        tokio::time::timeout(OPERATION_TIMEOUT, async {
            let (database, connection) = open_database(path).await?;
            if checkpoint_before_read {
                checkpoint(&connection).await?;
            }
            let health = read_health(&connection).await?;
            drop(connection);
            drop(database);
            Ok(health)
        })
        .await
        .map_err(|_| PersistenceError::Timeout)?
    })
}

fn copy_atomic(source: &Path, destination: &Path) -> Result<(u64, [u8; 32]), PersistenceError> {
    let source_bytes = source
        .metadata()
        .map_err(|_| PersistenceError::BackupIo)?
        .len();
    if source_bytes > MAX_BACKUP_BYTES {
        return Err(PersistenceError::BackupTooLarge);
    }
    let temporary = temporary_path(destination);
    let result = (|| {
        let source = File::open(source).map_err(|_| PersistenceError::BackupIo)?;
        let target = create_private_file(&temporary)?;
        let mut reader = BufReader::with_capacity(COPY_BUFFER_BYTES, source);
        let mut writer = BufWriter::with_capacity(COPY_BUFFER_BYTES, target);
        let mut hasher = Sha256::new();
        let mut copied = 0_u64;
        let mut buffer = [0_u8; COPY_BUFFER_BYTES];
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|_| PersistenceError::BackupIo)?;
            if read == 0 {
                break;
            }
            copied = copied
                .checked_add(read as u64)
                .ok_or(PersistenceError::BackupTooLarge)?;
            if copied > MAX_BACKUP_BYTES {
                return Err(PersistenceError::BackupTooLarge);
            }
            hasher.update(&buffer[..read]);
            writer
                .write_all(&buffer[..read])
                .map_err(|_| PersistenceError::BackupIo)?;
        }
        writer.flush().map_err(|_| PersistenceError::BackupIo)?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|_| PersistenceError::BackupIo)?;
        if copied != source_bytes {
            return Err(PersistenceError::BackupVerification);
        }
        fs::rename(&temporary, destination).map_err(|_| PersistenceError::BackupIo)?;
        let digest: [u8; 32] = hasher.finalize().into();
        Ok((copied, digest))
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn verify_file(path: &Path, manifest: BackupManifest) -> Result<(), PersistenceError> {
    let metadata = path.metadata().map_err(|_| PersistenceError::BackupIo)?;
    if metadata.len() != manifest.database_bytes || metadata.len() > MAX_BACKUP_BYTES {
        return Err(PersistenceError::BackupVerification);
    }
    let mut reader = BufReader::with_capacity(
        COPY_BUFFER_BYTES,
        File::open(path).map_err(|_| PersistenceError::BackupIo)?,
    );
    let mut hasher = Sha256::new();
    let mut copied = 0_u64;
    let mut buffer = [0_u8; COPY_BUFFER_BYTES];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| PersistenceError::BackupIo)?;
        if read == 0 {
            break;
        }
        copied = copied
            .checked_add(read as u64)
            .ok_or(PersistenceError::BackupTooLarge)?;
        hasher.update(&buffer[..read]);
    }
    let digest: [u8; 32] = hasher.finalize().into();
    if copied != manifest.database_bytes || digest != manifest.sha256 {
        return Err(PersistenceError::BackupVerification);
    }
    Ok(())
}

fn write_manifest_atomic(path: &Path, manifest: BackupManifest) -> Result<(), PersistenceError> {
    let temporary = temporary_path(path);
    let result = (|| {
        let mut file = create_private_file(&temporary)?;
        writeln!(file, "format={}", manifest.format_version)
            .map_err(|_| PersistenceError::BackupIo)?;
        writeln!(file, "schema={}", manifest.schema_version)
            .map_err(|_| PersistenceError::BackupIo)?;
        writeln!(file, "bytes={}", manifest.database_bytes)
            .map_err(|_| PersistenceError::BackupIo)?;
        writeln!(file, "sha256={}", encode_hex(manifest.sha256))
            .map_err(|_| PersistenceError::BackupIo)?;
        file.sync_all().map_err(|_| PersistenceError::BackupIo)?;
        fs::rename(&temporary, path).map_err(|_| PersistenceError::BackupIo)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn read_manifest(path: &Path) -> Result<BackupManifest, PersistenceError> {
    let metadata = path
        .metadata()
        .map_err(|_| PersistenceError::BackupManifest)?;
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Err(PersistenceError::BackupManifest);
    }
    let text = fs::read_to_string(path).map_err(|_| PersistenceError::BackupManifest)?;
    let mut lines = text.lines();
    let format_version = parse_u32(lines.next(), "format")?;
    let schema_version = parse_u32(lines.next(), "schema")?;
    let database_bytes = parse_u64(lines.next(), "bytes")?;
    let sha256 = parse_digest(lines.next())?;
    if lines.next().is_some()
        || format_version != BACKUP_FORMAT_VERSION
        || database_bytes > MAX_BACKUP_BYTES
    {
        return Err(PersistenceError::BackupManifest);
    }
    Ok(BackupManifest {
        format_version,
        schema_version,
        database_bytes,
        sha256,
    })
}

fn parse_u32(line: Option<&str>, key: &str) -> Result<u32, PersistenceError> {
    parse_value(line, key)?
        .parse()
        .map_err(|_| PersistenceError::BackupManifest)
}

fn parse_u64(line: Option<&str>, key: &str) -> Result<u64, PersistenceError> {
    parse_value(line, key)?
        .parse()
        .map_err(|_| PersistenceError::BackupManifest)
}

fn parse_value<'a>(line: Option<&'a str>, key: &str) -> Result<&'a str, PersistenceError> {
    line.and_then(|line| line.strip_prefix(key))
        .and_then(|value| value.strip_prefix('='))
        .ok_or(PersistenceError::BackupManifest)
}

fn parse_digest(line: Option<&str>) -> Result<[u8; 32], PersistenceError> {
    let value = parse_value(line, "sha256")?;
    if value.len() != 64 {
        return Err(PersistenceError::BackupManifest);
    }
    let mut digest = [0_u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| PersistenceError::BackupManifest)?;
    }
    Ok(digest)
}

fn encode_hex(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn manifest_path(backup: &Path) -> PathBuf {
    let mut value = backup.as_os_str().to_owned();
    value.push(".manifest");
    PathBuf::from(value)
}

fn temporary_path(destination: &Path) -> PathBuf {
    let mut value = destination.as_os_str().to_owned();
    value.push(format!(".partial-{}", std::process::id()));
    PathBuf::from(value)
}

fn sync_parent(path: &Path) -> Result<(), PersistenceError> {
    let parent = path.parent().ok_or(PersistenceError::InvalidPath)?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| PersistenceError::BackupIo)
}

fn create_private_file(path: &Path) -> Result<File, PersistenceError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map_err(|_| PersistenceError::BackupIo)
}
