//! Local SQLite file sessions (sample database + absolute file paths).

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use tablerock_core::{
    BoundedText, ByteLimit, CancelDispatch, CatalogChildrenState, CatalogNodeKind, ColumnMetadata,
    Engine, EngineType, OperationId, OwnedValue, PageDelivery, PageFacts, PageIdentity, PageLimits,
    PageWarnings, ResultPage, RowTotal, SAMPLE_SCHEMA_SQL, Truncation,
};
use tokio::sync::Mutex;

use crate::{
    AdapterError, AdapterFailureClass, CatalogExactness, CatalogNodeSeed, CatalogRequest,
    CatalogSubtree, DriverFuture, DriverPageRequest, DriverPageStream, DriverSession,
    ServerDescribe, SessionHealth,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqliteError {
    Path,
    Open(String),
    Query(String),
}

impl std::fmt::Display for SqliteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Path => f.write_str("sqlite path missing or not absolute"),
            Self::Open(detail) => write!(f, "sqlite open failed: {detail}"),
            Self::Query(detail) => write!(f, "sqlite query failed: {detail}"),
        }
    }
}

impl std::error::Error for SqliteError {}

impl From<SqliteError> for AdapterError {
    fn from(value: SqliteError) -> Self {
        let class = match value {
            SqliteError::Path | SqliteError::Open(_) => AdapterFailureClass::Connection,
            SqliteError::Query(_) => AdapterFailureClass::Query,
        };
        AdapterError::new(Engine::Sqlite, class)
    }
}

/// Ensure parent dirs exist and sample schema is applied once.
pub async fn ensure_sample_sqlite_database(path: &Path) -> Result<(), SqliteError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SqliteError::Open(e.to_string()))?;
    }
    let path_str = path.to_str().ok_or(SqliteError::Path)?;
    let database = turso::Builder::new_local(path_str)
        .build()
        .await
        .map_err(|e| SqliteError::Open(e.to_string()))?;
    let connection = database
        .connect()
        .map_err(|e| SqliteError::Open(e.to_string()))?;
    connection
        .execute_batch(SAMPLE_SCHEMA_SQL)
        .await
        .map_err(|e| SqliteError::Query(e.to_string()))?;
    // Checkpoint so a bare file copy (no -wal) still has seeded tables.
    let _ = connection
        .execute("PRAGMA wal_checkpoint(TRUNCATE);", ())
        .await;
    Ok(())
}

pub struct SqliteSession {
    path: PathBuf,
    connection: Arc<Mutex<turso::Connection>>,
}

impl SqliteSession {
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self, SqliteError> {
        let path = path.as_ref().to_path_buf();
        if !path.is_absolute() {
            return Err(SqliteError::Path);
        }
        let path_str = path.to_str().ok_or(SqliteError::Path)?;
        let database = turso::Builder::new_local(path_str)
            .build()
            .await
            .map_err(|e| SqliteError::Open(e.to_string()))?;
        let connection = database
            .connect()
            .map_err(|e| SqliteError::Open(e.to_string()))?;
        Ok(Self {
            path,
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    async fn list_tables(&self, limit: u32) -> Result<Vec<String>, SqliteError> {
        let conn = self.connection.lock().await;
        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type IN ('table','view') \
                 AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT ?1",
                (i64::from(limit),),
            )
            .await
            .map_err(|e| SqliteError::Query(e.to_string()))?;
        let mut names = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| SqliteError::Query(e.to_string()))?
        {
            let name: String = row.get(0).map_err(|e| SqliteError::Query(e.to_string()))?;
            names.push(name);
        }
        Ok(names)
    }

    async fn list_columns(
        &self,
        table: &str,
        limit: u32,
    ) -> Result<Vec<(String, String)>, SqliteError> {
        if !table.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || table.is_empty() {
            return Err(SqliteError::Query("invalid table name".into()));
        }
        let sql = format!("PRAGMA table_info({table})");
        let conn = self.connection.lock().await;
        let mut rows = conn
            .query(&sql, ())
            .await
            .map_err(|e| SqliteError::Query(e.to_string()))?;
        let mut cols = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| SqliteError::Query(e.to_string()))?
        {
            if cols.len() as u32 >= limit {
                break;
            }
            let name: String = row.get(1).map_err(|e| SqliteError::Query(e.to_string()))?;
            let ty: String = row.get::<String>(2).unwrap_or_else(|_| "TEXT".into());
            cols.push((name, ty));
        }
        Ok(cols)
    }

    /// Run a bounded read for sample/demo verification (not the paged workbench path).
    pub async fn query_rows(
        &self,
        sql: &str,
        max_rows: u32,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), SqliteError> {
        let conn = self.connection.lock().await;
        let mut rows = conn
            .query(sql, ())
            .await
            .map_err(|e| SqliteError::Query(e.to_string()))?;
        let mut headers: Vec<String> = Vec::new();
        let mut body: Vec<Vec<String>> = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| SqliteError::Query(e.to_string()))?
        {
            if headers.is_empty() {
                for i in 0..32_usize {
                    let ok = row.get::<String>(i).is_ok()
                        || row.get::<i64>(i).is_ok()
                        || row.get::<f64>(i).is_ok();
                    if ok {
                        headers.push(format!("c{i}"));
                    } else {
                        break;
                    }
                }
            }
            let width = headers.len().max(1);
            let mut cells = Vec::with_capacity(width);
            for i in 0..width {
                let cell = row
                    .get::<String>(i)
                    .or_else(|_| row.get::<i64>(i).map(|n| n.to_string()))
                    .or_else(|_| row.get::<f64>(i).map(|n| n.to_string()))
                    .unwrap_or_default();
                cells.push(cell);
            }
            body.push(cells);
            if body.len() as u32 >= max_rows {
                break;
            }
        }
        if headers.is_empty() && !body.is_empty() {
            headers = (0..body[0].len()).map(|i| format!("c{i}")).collect();
        }
        if headers.is_empty() {
            headers.push("result".into());
        }
        Ok((headers, body))
    }
}

impl DriverSession for SqliteSession {
    fn engine(&self) -> Engine {
        Engine::Sqlite
    }

    fn start_page_stream<'a>(
        &'a self,
        request: DriverPageRequest,
    ) -> DriverFuture<'a, Result<Box<dyn DriverPageStream>, AdapterError>> {
        Box::pin(async move {
            let (sql, limits, max_cell) = match request {
                DriverPageRequest::SqliteStatement {
                    statement,
                    limits,
                    max_cell_bytes,
                } => (statement.as_str().to_owned(), limits, max_cell_bytes),
                _ => {
                    return Err(AdapterError::new(
                        Engine::Sqlite,
                        AdapterFailureClass::EngineMismatch,
                    ));
                }
            };
            let max_rows = limits.max_rows().min(500);
            let (headers, body) = self
                .query_rows(&sql, max_rows)
                .await
                .map_err(AdapterError::from)?;
            Ok(Box::new(SqlitePageStream {
                headers,
                body,
                limits,
                max_cell_bytes: max_cell,
                delivered: false,
            }) as Box<dyn DriverPageStream>)
        })
    }

    fn cancel<'a>(&'a self, _operation_id: OperationId) -> DriverFuture<'a, CancelDispatch> {
        Box::pin(async { CancelDispatch::RequestSent })
    }

    fn shutdown(self: Box<Self>) -> DriverFuture<'static, Result<(), AdapterError>> {
        Box::pin(async { Ok(()) })
    }

    fn health<'a>(&'a self) -> DriverFuture<'a, Result<SessionHealth, AdapterError>> {
        Box::pin(async {
            let started = Instant::now();
            let conn = self.connection.lock().await;
            let mut rows = conn
                .query("SELECT 1", ())
                .await
                .map_err(|e| AdapterError::from(SqliteError::Query(e.to_string())))?;
            let _ = rows
                .next()
                .await
                .map_err(|e| AdapterError::from(SqliteError::Query(e.to_string())))?;
            Ok(SessionHealth::new(
                Engine::Sqlite,
                true,
                u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            ))
        })
    }

    fn catalog<'a>(
        &'a self,
        request: CatalogRequest,
    ) -> DriverFuture<'a, Result<CatalogSubtree, AdapterError>> {
        Box::pin(async move {
            match request {
                CatalogRequest::SqliteRoot { limits: _ } => {
                    let name = self
                        .path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("sqlite");
                    let seed = CatalogNodeSeed::new(
                        CatalogNodeKind::SqliteDatabase,
                        BoundedText::copy_from_str(name, ByteLimit::new(256))
                            .map_err(|_| AdapterError::from(SqliteError::Path))?,
                        CatalogChildrenState::Unrequested,
                        None,
                    );
                    Ok(CatalogSubtree::new(
                        Engine::Sqlite,
                        vec![seed],
                        true,
                        CatalogExactness::Exact,
                    ))
                }
                CatalogRequest::SqliteTables { limits } => {
                    let tables = self
                        .list_tables(limits.max_rows().min(500))
                        .await
                        .map_err(AdapterError::from)?;
                    let seeds = tables
                        .into_iter()
                        .filter_map(|name| {
                            let text =
                                BoundedText::copy_from_str(&name, ByteLimit::new(256)).ok()?;
                            Some(CatalogNodeSeed::new(
                                CatalogNodeKind::SqliteTable,
                                text,
                                CatalogChildrenState::Unrequested,
                                None,
                            ))
                        })
                        .collect();
                    Ok(CatalogSubtree::new(
                        Engine::Sqlite,
                        seeds,
                        true,
                        CatalogExactness::Exact,
                    ))
                }
                CatalogRequest::SqliteColumns { table, limits } => {
                    let cols = self
                        .list_columns(table.as_str(), limits.max_rows().min(500))
                        .await
                        .map_err(AdapterError::from)?;
                    let seeds = cols
                        .into_iter()
                        .filter_map(|(name, ty)| {
                            let text =
                                BoundedText::copy_from_str(&name, ByteLimit::new(256)).ok()?;
                            let engine_type = EngineType::new(
                                Engine::Sqlite,
                                BoundedText::copy_from_str(&ty, ByteLimit::new(64)).ok()?,
                            )
                            .ok();
                            Some(CatalogNodeSeed::new(
                                CatalogNodeKind::SqliteColumn,
                                text,
                                CatalogChildrenState::NotApplicable,
                                engine_type,
                            ))
                        })
                        .collect();
                    Ok(CatalogSubtree::new(
                        Engine::Sqlite,
                        seeds,
                        true,
                        CatalogExactness::Exact,
                    ))
                }
                _ => Err(AdapterError::new(
                    Engine::Sqlite,
                    AdapterFailureClass::EngineMismatch,
                )),
            }
        })
    }

    fn describe<'a>(&'a self) -> DriverFuture<'a, Result<ServerDescribe, AdapterError>> {
        Box::pin(async {
            Ok(ServerDescribe::new(
                Engine::Sqlite,
                format!("SQLite local {}", self.path.display()),
                0,
            ))
        })
    }
}

struct SqlitePageStream {
    headers: Vec<String>,
    body: Vec<Vec<String>>,
    limits: PageLimits,
    max_cell_bytes: u64,
    delivered: bool,
}

impl DriverPageStream for SqlitePageStream {
    fn next_page<'a>(
        &'a mut self,
        identity: PageIdentity,
        start_row: u64,
    ) -> DriverFuture<'a, Result<Option<ResultPage>, AdapterError>> {
        Box::pin(async move {
            if self.delivered {
                return Ok(None);
            }
            self.delivered = true;
            let page = build_text_page(
                identity,
                start_row,
                &self.headers,
                &self.body,
                self.limits,
                self.max_cell_bytes,
            )?;
            Ok(Some(page))
        })
    }
}

fn build_text_page(
    identity: PageIdentity,
    start_row: u64,
    headers: &[String],
    body: &[Vec<String>],
    limits: PageLimits,
    max_cell_bytes: u64,
) -> Result<ResultPage, AdapterError> {
    let mut columns = Vec::with_capacity(headers.len());
    for name in headers {
        let text = BoundedText::copy_from_str(name, ByteLimit::new(256))
            .map_err(|_| AdapterError::new(Engine::Sqlite, AdapterFailureClass::Query))?;
        let engine_type = EngineType::new(
            Engine::Sqlite,
            BoundedText::copy_from_str("TEXT", ByteLimit::new(16))
                .map_err(|_| AdapterError::new(Engine::Sqlite, AdapterFailureClass::Query))?,
        )
        .map_err(|_| AdapterError::new(Engine::Sqlite, AdapterFailureClass::Query))?;
        columns.push(ColumnMetadata::new(text, engine_type, true));
    }
    let mut values = Vec::new();
    for row in body {
        for cell in row {
            let clipped = if cell.len() as u64 > max_cell_bytes {
                let end = (max_cell_bytes as usize).min(cell.len());
                &cell[..end]
            } else {
                cell.as_str()
            };
            let text =
                BoundedText::copy_from_str(clipped, ByteLimit::new(max_cell_bytes.max(1)))
                    .map_err(|_| AdapterError::new(Engine::Sqlite, AdapterFailureClass::Query))?;
            let trunc = if (cell.len() as u64) > max_cell_bytes {
                Truncation::Truncated {
                    original_byte_len: Some(cell.len() as u64),
                }
            } else {
                Truncation::Complete
            };
            values.push(
                OwnedValue::text(text, trunc)
                    .map_err(|_| AdapterError::new(Engine::Sqlite, AdapterFailureClass::Query))?,
            );
        }
    }
    ResultPage::from_row_major(
        identity,
        start_row,
        RowTotal::Known(body.len() as u64),
        PageFacts::new(PageDelivery::Final, PageWarnings::none()),
        columns,
        values,
        limits,
    )
    .map_err(|_| AdapterError::new(Engine::Sqlite, AdapterFailureClass::Query))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn sample_fixture_lists_tables_and_runs_select() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("tablerock-sample-test-{nanos}.db"));
        let _ = std::fs::remove_file(&path);
        ensure_sample_sqlite_database(&path).await.unwrap();
        let session = SqliteSession::connect(&path).await.unwrap();
        let tables = session.list_tables(50).await.unwrap();
        assert!(tables.iter().any(|t| t == "artists"));
        assert!(tables.iter().any(|t| t == "tracks"));
        let (headers, body) = session
            .query_rows("SELECT name FROM artists ORDER BY id", 10)
            .await
            .unwrap();
        assert!(!headers.is_empty());
        assert!(
            body.iter()
                .any(|row| row.iter().any(|c| c.contains("Northwind")))
        );
        let _ = std::fs::remove_file(&path);
    }
}
