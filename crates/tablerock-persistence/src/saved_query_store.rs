//! Named saved queries (statement text only; never results).

use tablerock_core::Engine;

use crate::PersistenceError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedQuery {
    pub query_id: i64,
    pub name: String,
    pub engine: Engine,
    pub statement_text: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedQueryUpsert {
    pub name: String,
    pub engine: Engine,
    pub statement_text: String,
}

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

pub async fn upsert(
    connection: &turso::Connection,
    request: &SavedQueryUpsert,
) -> Result<i64, PersistenceError> {
    let name = request.name.trim();
    let text = request.statement_text.trim();
    if name.is_empty() || text.is_empty() {
        return Err(PersistenceError::Query);
    }
    if name.len() > 128 || text.len() > 1_048_576 {
        return Err(PersistenceError::Query);
    }
    connection
        .execute(
            "INSERT INTO saved_queries(name, engine, statement_text, updated_at)
             VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)
             ON CONFLICT(name, engine) DO UPDATE SET
               statement_text = excluded.statement_text,
               updated_at = CURRENT_TIMESTAMP",
            (name, engine_code(request.engine), text),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    // Resolve id by unique key.
    let mut rows = connection
        .query(
            "SELECT query_id FROM saved_queries WHERE name = ?1 AND engine = ?2",
            (name, engine_code(request.engine)),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    let Some(row) = rows.next().await.map_err(|_| PersistenceError::Query)? else {
        return Err(PersistenceError::Query);
    };
    row.get::<i64>(0).map_err(|_| PersistenceError::Query)
}

pub async fn list(
    connection: &turso::Connection,
    engine: Option<Engine>,
) -> Result<Vec<SavedQuery>, PersistenceError> {
    let mut out = Vec::new();
    if let Some(engine) = engine {
        let mut rows = connection
            .query(
                "SELECT query_id, name, engine, statement_text, updated_at
                 FROM saved_queries WHERE engine = ?1
                 ORDER BY name COLLATE NOCASE",
                (engine_code(engine),),
            )
            .await
            .map_err(|_| PersistenceError::Query)?;
        while let Some(row) = rows.next().await.map_err(|_| PersistenceError::Query)? {
            out.push(row_to_query(&row)?);
        }
    } else {
        let mut rows = connection
            .query(
                "SELECT query_id, name, engine, statement_text, updated_at
                 FROM saved_queries ORDER BY name COLLATE NOCASE",
                (),
            )
            .await
            .map_err(|_| PersistenceError::Query)?;
        while let Some(row) = rows.next().await.map_err(|_| PersistenceError::Query)? {
            out.push(row_to_query(&row)?);
        }
    }
    Ok(out)
}

pub async fn get(
    connection: &turso::Connection,
    query_id: i64,
) -> Result<Option<SavedQuery>, PersistenceError> {
    let mut rows = connection
        .query(
            "SELECT query_id, name, engine, statement_text, updated_at
             FROM saved_queries WHERE query_id = ?1",
            (query_id,),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    let Some(row) = rows.next().await.map_err(|_| PersistenceError::Query)? else {
        return Ok(None);
    };
    Ok(Some(row_to_query(&row)?))
}

pub async fn delete(
    connection: &turso::Connection,
    query_id: i64,
) -> Result<bool, PersistenceError> {
    connection
        .execute("DELETE FROM saved_queries WHERE query_id = ?1", (query_id,))
        .await
        .map_err(|_| PersistenceError::Query)?;
    // turso may not expose rows_affected; treat as best-effort success.
    Ok(true)
}

fn row_to_query(row: &turso::Row) -> Result<SavedQuery, PersistenceError> {
    Ok(SavedQuery {
        query_id: row.get::<i64>(0).map_err(|_| PersistenceError::Query)?,
        name: row.get::<String>(1).map_err(|_| PersistenceError::Query)?,
        engine: engine_from_code(row.get::<i64>(2).map_err(|_| PersistenceError::Query)?),
        statement_text: row.get::<String>(3).map_err(|_| PersistenceError::Query)?,
        updated_at: row.get::<String>(4).map_err(|_| PersistenceError::Query)?,
    })
}
