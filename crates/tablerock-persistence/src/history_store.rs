//! Bounded query history (statement text optional by retention).

use tablerock_core::Engine;

use crate::PersistenceError;

/// How statement text is retained for history entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryRetention {
    /// Store full statement text (bounded length).
    #[default]
    Full,
    /// Append metadata only; statement_text is NULL.
    MetadataOnly,
    /// Do not append history at all.
    Private,
}

/// Outcome class stored with a history row (never result payloads).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryOutcomeClass {
    Completed,
    Cancelled,
    Failed,
    Disconnected,
    Unknown,
}

impl HistoryOutcomeClass {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
            Self::Disconnected => "disconnected",
            Self::Unknown => "unknown",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "completed" => Self::Completed,
            "cancelled" => Self::Cancelled,
            "failed" => Self::Failed,
            "disconnected" => Self::Disconnected,
            _ => Self::Unknown,
        }
    }
}

/// One append request from the effect layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryAppend {
    pub engine: Engine,
    pub database_name: String,
    pub schema_name: Option<String>,
    pub statement_text: String,
    pub outcome: HistoryOutcomeClass,
    pub retention: HistoryRetention,
}

/// One history list/search row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub history_id: i64,
    pub engine: Engine,
    pub database_name: String,
    pub schema_name: Option<String>,
    pub statement_text: Option<String>,
    pub outcome: HistoryOutcomeClass,
    pub created_at: String,
}

/// Default max rows retained (oldest deleted on overflow).
pub const DEFAULT_HISTORY_LIMIT: u32 = 500;

fn engine_code(engine: Engine) -> i64 {
    match engine {
        Engine::PostgreSql => 1,
        Engine::ClickHouse => 2,
        Engine::Redis => 3,
    }
}

fn engine_from_code(code: i64) -> Engine {
    match code {
        2 => Engine::ClickHouse,
        3 => Engine::Redis,
        _ => Engine::PostgreSql,
    }
}

pub async fn append(
    connection: &turso::Connection,
    request: &HistoryAppend,
    max_rows: u32,
) -> Result<Option<i64>, PersistenceError> {
    if matches!(request.retention, HistoryRetention::Private) {
        return Ok(None);
    }
    let text = match request.retention {
        HistoryRetention::Full => {
            let t = request.statement_text.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.chars().take(1_048_576).collect::<String>())
            }
        }
        HistoryRetention::MetadataOnly | HistoryRetention::Private => None,
    };
    connection
        .execute(
            "INSERT INTO query_history(
                engine, database_name, schema_name, statement_text, outcome_class
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                engine_code(request.engine),
                request.database_name.as_str(),
                request.schema_name.as_deref(),
                text.as_deref(),
                request.outcome.as_str(),
            ),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    let id = connection.last_insert_rowid();
    enforce_limit(connection, max_rows).await?;
    Ok(Some(id))
}

async fn enforce_limit(
    connection: &turso::Connection,
    max_rows: u32,
) -> Result<(), PersistenceError> {
    if max_rows == 0 {
        return Ok(());
    }
    let mut rows = connection
        .query(
            "SELECT COUNT(*) FROM query_history",
            (),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    let count = if let Some(row) = rows.next().await.map_err(|_| PersistenceError::Query)? {
        row.get::<i64>(0).map_err(|_| PersistenceError::Query)?
    } else {
        0
    };
    let excess = count.saturating_sub(i64::from(max_rows));
    if excess > 0 {
        connection
            .execute(
                "DELETE FROM query_history WHERE history_id IN (
                    SELECT history_id FROM query_history
                    ORDER BY history_id ASC
                    LIMIT ?1
                )",
                (excess,),
            )
            .await
            .map_err(|_| PersistenceError::Query)?;
    }
    Ok(())
}

pub async fn list(
    connection: &turso::Connection,
    search: Option<&str>,
    limit: u32,
) -> Result<Vec<HistoryEntry>, PersistenceError> {
    let limit = limit.clamp(1, 500);
    let mut entries = Vec::new();
    if let Some(term) = search.filter(|s| !s.trim().is_empty()) {
        let like = format!("%{}%", term.trim());
        let mut rows = connection
            .query(
                "SELECT history_id, engine, database_name, schema_name,
                        statement_text, outcome_class, created_at
                 FROM query_history
                 WHERE statement_text IS NOT NULL AND statement_text LIKE ?1
                 ORDER BY history_id DESC
                 LIMIT ?2",
                (like.as_str(), i64::from(limit)),
            )
            .await
            .map_err(|_| PersistenceError::Query)?;
        while let Some(row) = rows.next().await.map_err(|_| PersistenceError::Query)? {
            entries.push(row_to_entry(&row)?);
        }
    } else {
        let mut rows = connection
            .query(
                "SELECT history_id, engine, database_name, schema_name,
                        statement_text, outcome_class, created_at
                 FROM query_history
                 ORDER BY history_id DESC
                 LIMIT ?1",
                (i64::from(limit),),
            )
            .await
            .map_err(|_| PersistenceError::Query)?;
        while let Some(row) = rows.next().await.map_err(|_| PersistenceError::Query)? {
            entries.push(row_to_entry(&row)?);
        }
    }
    Ok(entries)
}

fn row_to_entry(row: &turso::Row) -> Result<HistoryEntry, PersistenceError> {
    let history_id = row.get::<i64>(0).map_err(|_| PersistenceError::Query)?;
    let engine = engine_from_code(row.get::<i64>(1).map_err(|_| PersistenceError::Query)?);
    let database_name = row.get::<String>(2).map_err(|_| PersistenceError::Query)?;
    let schema_name = row.get::<Option<String>>(3).map_err(|_| PersistenceError::Query)?;
    let statement_text = row.get::<Option<String>>(4).map_err(|_| PersistenceError::Query)?;
    let outcome = HistoryOutcomeClass::parse(
        &row.get::<String>(5).map_err(|_| PersistenceError::Query)?,
    );
    let created_at = row.get::<String>(6).map_err(|_| PersistenceError::Query)?;
    Ok(HistoryEntry {
        history_id,
        engine,
        database_name,
        schema_name,
        statement_text,
        outcome,
        created_at,
    })
}

pub async fn count(connection: &turso::Connection) -> Result<u64, PersistenceError> {
    let mut rows = connection
        .query("SELECT COUNT(*) FROM query_history", ())
        .await
        .map_err(|_| PersistenceError::Query)?;
    let Some(row) = rows.next().await.map_err(|_| PersistenceError::Query)? else {
        return Ok(0);
    };
    let n = row.get::<i64>(0).map_err(|_| PersistenceError::Query)?;
    Ok(u64::try_from(n).unwrap_or(0))
}
