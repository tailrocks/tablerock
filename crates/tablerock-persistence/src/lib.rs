//! Local-only persistence owned by one serialized worker thread.

mod profile_store;

use std::{
    collections::HashSet,
    error::Error,
    fmt,
    path::{Path, PathBuf},
    sync::{
        Mutex, OnceLock,
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use profile_store::EncodedProfile;
use tablerock_core::{
    PersistableProfile, ProfileAggregate, ProfileId, ProfileListPage, ProfileListRequest, Revision,
};

const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
const CALLER_TIMEOUT: Duration = Duration::from_secs(35);

const MIGRATIONS: &[(u32, &str)] = &[
    (1, include_str!("../migrations/0001-bootstrap.sql")),
    (2, include_str!("../migrations/0002-support-facts.sql")),
    (3, include_str!("../migrations/0003-saved-profiles.sql")),
    (4, include_str!("../migrations/0004-profile-list-index.sql")),
    (
        5,
        include_str!("../migrations/0005-profile-engine-list-index.sql"),
    ),
    (
        6,
        include_str!("../migrations/0006-profile-group-list-index.sql"),
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersistenceHealth {
    pub schema_version: u32,
    pub foreign_keys_enabled: bool,
    pub integrity_ok: bool,
}

pub struct PersistenceActor {
    sender: SyncSender<Command>,
    worker: Option<JoinHandle<()>>,
}

impl PersistenceActor {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PersistenceError> {
        let path = normalize_database_path(path.as_ref())?;
        let lease = PathLease::acquire(path.clone())?;
        let (sender, receiver) = mpsc::sync_channel(32);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("tablerock-persistence".to_owned())
            .spawn(move || worker_main(path, receiver, ready_sender, lease))
            .map_err(|_| PersistenceError::WorkerStart)?;
        match ready_receiver.recv_timeout(CALLER_TIMEOUT) {
            Ok(Ok(())) => Ok(Self {
                sender,
                worker: Some(worker),
            }),
            Ok(Err(error)) => {
                drop(worker);
                Err(error)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => Err(PersistenceError::Timeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(PersistenceError::WorkerStopped),
        }
    }

    pub fn health(&self) -> Result<PersistenceHealth, PersistenceError> {
        let (sender, receiver) = mpsc::sync_channel(1);
        submit(&self.sender, Command::Health(sender))?;
        receiver
            .recv_timeout(CALLER_TIMEOUT)
            .map_err(map_receive_error)?
    }

    pub fn create_profile(&self, profile: PersistableProfile<'_>) -> Result<(), PersistenceError> {
        let profile = EncodedProfile::from_saved(profile);
        let (sender, receiver) = mpsc::sync_channel(1);
        submit(&self.sender, Command::CreateProfile(profile, sender))?;
        receiver
            .recv_timeout(CALLER_TIMEOUT)
            .map_err(map_receive_error)?
    }

    pub fn get_profile(&self, id: ProfileId) -> Result<Option<ProfileAggregate>, PersistenceError> {
        let (sender, receiver) = mpsc::sync_channel(1);
        submit(&self.sender, Command::GetProfile(id, sender))?;
        receiver
            .recv_timeout(CALLER_TIMEOUT)
            .map_err(map_receive_error)?
    }

    pub fn replace_profile(
        &self,
        expected_revision: Revision,
        profile: PersistableProfile<'_>,
    ) -> Result<(), PersistenceError> {
        let profile = EncodedProfile::from_saved(profile);
        let (sender, receiver) = mpsc::sync_channel(1);
        submit(
            &self.sender,
            Command::ReplaceProfile(expected_revision, profile, sender),
        )?;
        receiver
            .recv_timeout(CALLER_TIMEOUT)
            .map_err(map_receive_error)?
    }

    pub fn delete_profile(
        &self,
        id: ProfileId,
        expected_revision: Revision,
    ) -> Result<(), PersistenceError> {
        let (sender, receiver) = mpsc::sync_channel(1);
        submit(
            &self.sender,
            Command::DeleteProfile(id, expected_revision, sender),
        )?;
        receiver
            .recv_timeout(CALLER_TIMEOUT)
            .map_err(map_receive_error)?
    }

    pub fn list_profiles(
        &self,
        request: ProfileListRequest,
    ) -> Result<ProfileListPage, PersistenceError> {
        let (sender, receiver) = mpsc::sync_channel(1);
        submit(&self.sender, Command::ListProfiles(request, sender))?;
        receiver
            .recv_timeout(CALLER_TIMEOUT)
            .map_err(map_receive_error)?
    }

    pub fn shutdown(mut self) -> Result<(), PersistenceError> {
        let (sender, receiver) = mpsc::sync_channel(1);
        submit(&self.sender, Command::Shutdown(sender))?;
        let result = receiver
            .recv_timeout(CALLER_TIMEOUT)
            .map_err(map_receive_error)?;
        drop(self.worker.take());
        result
    }
}

impl Drop for PersistenceActor {
    fn drop(&mut self) {
        if self.worker.take().is_some() {
            let (sender, _receiver) = mpsc::sync_channel(1);
            let _ = self.sender.try_send(Command::Shutdown(sender));
        }
    }
}

fn submit(sender: &SyncSender<Command>, command: Command) -> Result<(), PersistenceError> {
    sender.try_send(command).map_err(|error| match error {
        TrySendError::Full(_) => PersistenceError::QueueFull,
        TrySendError::Disconnected(_) => PersistenceError::WorkerStopped,
    })
}

fn map_receive_error(error: mpsc::RecvTimeoutError) -> PersistenceError {
    match error {
        mpsc::RecvTimeoutError::Timeout => PersistenceError::Timeout,
        mpsc::RecvTimeoutError::Disconnected => PersistenceError::WorkerStopped,
    }
}

enum Command {
    Health(mpsc::SyncSender<Result<PersistenceHealth, PersistenceError>>),
    CreateProfile(
        EncodedProfile,
        mpsc::SyncSender<Result<(), PersistenceError>>,
    ),
    GetProfile(
        ProfileId,
        mpsc::SyncSender<Result<Option<ProfileAggregate>, PersistenceError>>,
    ),
    ReplaceProfile(
        Revision,
        EncodedProfile,
        mpsc::SyncSender<Result<(), PersistenceError>>,
    ),
    DeleteProfile(
        ProfileId,
        Revision,
        mpsc::SyncSender<Result<(), PersistenceError>>,
    ),
    ListProfiles(
        ProfileListRequest,
        mpsc::SyncSender<Result<ProfileListPage, PersistenceError>>,
    ),
    Shutdown(mpsc::SyncSender<Result<(), PersistenceError>>),
}

fn worker_main(
    path: PathBuf,
    receiver: Receiver<Command>,
    ready: mpsc::SyncSender<Result<(), PersistenceError>>,
    lease: PathLease,
) {
    let mut lease = Some(lease);
    let runtime = match tokio_runtime() {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };
    let opened = runtime.block_on(async {
        tokio::time::timeout(OPERATION_TIMEOUT, open_database(&path))
            .await
            .map_err(|_| PersistenceError::Timeout)?
    });
    let (database, mut connection) = match opened {
        Ok(value) => {
            let _ = ready.send(Ok(()));
            value
        }
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };
    for command in receiver {
        match command {
            Command::Health(reply) => {
                let result = runtime.block_on(async {
                    tokio::time::timeout(OPERATION_TIMEOUT, read_health(&connection))
                        .await
                        .map_err(|_| PersistenceError::Timeout)?
                });
                let _ = reply.send(result);
            }
            Command::CreateProfile(profile, reply) => {
                let result = runtime.block_on(async {
                    tokio::time::timeout(
                        OPERATION_TIMEOUT,
                        profile_store::create(&mut connection, &profile),
                    )
                    .await
                    .map_err(|_| PersistenceError::Timeout)?
                });
                let _ = reply.send(result);
            }
            Command::GetProfile(id, reply) => {
                let result = runtime.block_on(async {
                    tokio::time::timeout(
                        OPERATION_TIMEOUT,
                        profile_store::read(&mut connection, id),
                    )
                    .await
                    .map_err(|_| PersistenceError::Timeout)?
                });
                let _ = reply.send(result);
            }
            Command::ReplaceProfile(expected_revision, profile, reply) => {
                let result = runtime.block_on(async {
                    tokio::time::timeout(
                        OPERATION_TIMEOUT,
                        profile_store::replace(&mut connection, expected_revision, &profile),
                    )
                    .await
                    .map_err(|_| PersistenceError::Timeout)?
                });
                let _ = reply.send(result);
            }
            Command::DeleteProfile(id, expected_revision, reply) => {
                let result = runtime.block_on(async {
                    tokio::time::timeout(
                        OPERATION_TIMEOUT,
                        profile_store::delete(&mut connection, id, expected_revision),
                    )
                    .await
                    .map_err(|_| PersistenceError::Timeout)?
                });
                let _ = reply.send(result);
            }
            Command::ListProfiles(request, reply) => {
                let result = runtime.block_on(async {
                    tokio::time::timeout(
                        OPERATION_TIMEOUT,
                        profile_store::list(&mut connection, request),
                    )
                    .await
                    .map_err(|_| PersistenceError::Timeout)?
                });
                let _ = reply.send(result);
            }
            Command::Shutdown(reply) => {
                let result = runtime.block_on(async {
                    tokio::time::timeout(OPERATION_TIMEOUT, checkpoint(&connection))
                        .await
                        .map_err(|_| PersistenceError::Timeout)?
                });
                drop(connection);
                drop(database);
                drop(lease.take());
                let _ = reply.send(result);
                break;
            }
        }
    }
}

fn normalize_database_path(path: &Path) -> Result<PathBuf, PersistenceError> {
    if path.exists() {
        return path
            .canonicalize()
            .map_err(|_| PersistenceError::InvalidPath);
    }
    let file_name = path.file_name().ok_or(PersistenceError::InvalidPath)?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let parent = match parent {
        Some(parent) => parent
            .canonicalize()
            .map_err(|_| PersistenceError::InvalidPath)?,
        None => std::env::current_dir().map_err(|_| PersistenceError::InvalidPath)?,
    };
    Ok(parent.join(file_name))
}

fn leased_paths() -> &'static Mutex<HashSet<PathBuf>> {
    static LEASED_PATHS: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    LEASED_PATHS.get_or_init(|| Mutex::new(HashSet::new()))
}

struct PathLease {
    path: PathBuf,
}

impl PathLease {
    fn acquire(path: PathBuf) -> Result<Self, PersistenceError> {
        let mut paths = leased_paths()
            .lock()
            .map_err(|_| PersistenceError::OwnershipRegistry)?;
        if !paths.insert(path.clone()) {
            return Err(PersistenceError::DatabaseBusy);
        }
        Ok(Self { path })
    }
}

impl Drop for PathLease {
    fn drop(&mut self) {
        if let Ok(mut paths) = leased_paths().lock() {
            paths.remove(&self.path);
        }
    }
}

fn tokio_runtime() -> Result<tokio::runtime::Runtime, PersistenceError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| PersistenceError::RuntimeStart)
}

async fn open_database(
    path: &Path,
) -> Result<(turso::Database, turso::Connection), PersistenceError> {
    let path = path.to_str().ok_or(PersistenceError::InvalidPath)?;
    let database = turso::Builder::new_local(path)
        .build()
        .await
        .map_err(|_| PersistenceError::DatabaseOpen)?;
    let mut connection = database
        .connect()
        .map_err(|_| PersistenceError::DatabaseOpen)?;
    connection
        .pragma_update("foreign_keys", "ON")
        .await
        .map_err(|_| PersistenceError::Pragma)?;
    apply_migrations(&mut connection).await?;
    Ok((database, connection))
}

async fn apply_migrations(connection: &mut turso::Connection) -> Result<(), PersistenceError> {
    let ledger_exists = query_u32(
        connection,
        "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = 'schema_migrations'",
        (),
    )
    .await?
        == 1;
    if !ledger_exists {
        connection
            .execute_batch(MIGRATIONS[0].1)
            .await
            .map_err(|_| PersistenceError::Migration { version: 1 })?;
    }

    let applied = read_applied_versions(connection).await?;
    validate_migration_prefix(&applied)?;
    for &(version, sql) in MIGRATIONS.iter().skip(applied.len()) {
        if version > 1 {
            let transaction = connection
                .transaction()
                .await
                .map_err(|_| PersistenceError::Migration { version })?;
            transaction
                .execute_batch(sql)
                .await
                .map_err(|_| PersistenceError::Migration { version })?;
            transaction
                .execute(
                    "INSERT INTO schema_migrations(version) VALUES (?1)",
                    (version,),
                )
                .await
                .map_err(|_| PersistenceError::Migration { version })?;
            transaction
                .commit()
                .await
                .map_err(|_| PersistenceError::Migration { version })?;
        }
    }
    let applied = read_applied_versions(connection).await?;
    if applied.len() != MIGRATIONS.len() {
        return Err(PersistenceError::InvalidMigrationLedger);
    }
    validate_migration_prefix(&applied)?;
    Ok(())
}

async fn read_applied_versions(
    connection: &turso::Connection,
) -> Result<Vec<u32>, PersistenceError> {
    let mut rows = connection
        .query("SELECT version FROM schema_migrations ORDER BY version", ())
        .await
        .map_err(|_| PersistenceError::InvalidMigrationLedger)?;
    let mut versions = Vec::new();
    while let Some(row) = rows
        .next()
        .await
        .map_err(|_| PersistenceError::InvalidMigrationLedger)?
    {
        versions.push(
            row.get::<u32>(0)
                .map_err(|_| PersistenceError::InvalidMigrationLedger)?,
        );
    }
    Ok(versions)
}

fn validate_migration_prefix(applied: &[u32]) -> Result<(), PersistenceError> {
    if applied.is_empty()
        || applied.len() > MIGRATIONS.len()
        || applied
            .iter()
            .zip(MIGRATIONS)
            .any(|(actual, (expected, _))| actual != expected)
    {
        return Err(PersistenceError::InvalidMigrationLedger);
    }
    Ok(())
}

async fn read_health(
    connection: &turso::Connection,
) -> Result<PersistenceHealth, PersistenceError> {
    let schema_version = query_u32(
        connection,
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        (),
    )
    .await?;
    let foreign_keys_enabled = query_u32(connection, "PRAGMA foreign_keys", ()).await? == 1;
    let integrity = query_text(connection, "PRAGMA integrity_check", ()).await?;
    Ok(PersistenceHealth {
        schema_version,
        foreign_keys_enabled,
        integrity_ok: integrity == "ok",
    })
}

async fn checkpoint(connection: &turso::Connection) -> Result<(), PersistenceError> {
    let mut rows = connection
        .query("PRAGMA wal_checkpoint(TRUNCATE)", ())
        .await
        .map_err(|_| PersistenceError::Checkpoint)?;
    while rows
        .next()
        .await
        .map_err(|_| PersistenceError::Checkpoint)?
        .is_some()
    {}
    Ok(())
}

async fn query_u32(
    connection: &turso::Connection,
    sql: &str,
    params: impl turso::IntoParams,
) -> Result<u32, PersistenceError> {
    let mut rows = connection
        .query(sql, params)
        .await
        .map_err(|_| PersistenceError::Query)?;
    let row = rows
        .next()
        .await
        .map_err(|_| PersistenceError::Query)?
        .ok_or(PersistenceError::MissingRow)?;
    row.get::<u32>(0).map_err(|_| PersistenceError::Decode)
}

async fn query_text(
    connection: &turso::Connection,
    sql: &str,
    params: impl turso::IntoParams,
) -> Result<String, PersistenceError> {
    let mut rows = connection
        .query(sql, params)
        .await
        .map_err(|_| PersistenceError::Query)?;
    let row = rows
        .next()
        .await
        .map_err(|_| PersistenceError::Query)?
        .ok_or(PersistenceError::MissingRow)?;
    row.get::<String>(0).map_err(|_| PersistenceError::Decode)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistenceError {
    WorkerStart,
    WorkerStopped,
    QueueFull,
    RuntimeStart,
    InvalidPath,
    DatabaseBusy,
    OwnershipRegistry,
    DatabaseOpen,
    Pragma,
    Migration { version: u32 },
    InvalidMigrationLedger,
    Query,
    MissingRow,
    Decode,
    Checkpoint,
    ProfileAlreadyExists,
    ProfileWrite,
    ProfileRead,
    ProfileDecode,
    ProfileNotFound,
    ProfileStaleRevision,
    ProfileInvalidRevision,
    ProfileCapacity,
    Timeout,
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("local persistence operation failed")
    }
}

impl Error for PersistenceError {}
