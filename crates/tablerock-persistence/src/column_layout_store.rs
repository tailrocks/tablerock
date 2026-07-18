//! Per-table column layout persistence (visibility/order/width only).

use tablerock_core::ProfileId;

use crate::PersistenceError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnLayoutKey {
    pub profile_id: ProfileId,
    pub database: String,
    pub schema: String,
    pub table: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnLayoutRecord {
    pub key: ColumnLayoutKey,
    pub layout_json: String,
    pub updated_at: String,
}

pub async fn put(
    connection: &turso::Connection,
    key: &ColumnLayoutKey,
    layout_json: &str,
) -> Result<(), PersistenceError> {
    if layout_json.len() < 2 || layout_json.len() > 65_536 {
        return Err(PersistenceError::Query);
    }
    // Reject accidental value payloads.
    if layout_json.contains("\"cells\"") || layout_json.contains("\"result") {
        return Err(PersistenceError::Query);
    }
    let id = key.profile_id.to_bytes();
    connection
        .execute(
            "INSERT INTO column_layouts(
                profile_id, database_name, schema_name, table_name, layout_json, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP)
             ON CONFLICT(profile_id, database_name, schema_name, table_name) DO UPDATE SET
               layout_json = excluded.layout_json,
               updated_at = CURRENT_TIMESTAMP",
            (
                id.as_slice(),
                key.database.as_str(),
                key.schema.as_str(),
                key.table.as_str(),
                layout_json,
            ),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    Ok(())
}

pub async fn get(
    connection: &turso::Connection,
    key: &ColumnLayoutKey,
) -> Result<Option<ColumnLayoutRecord>, PersistenceError> {
    let id = key.profile_id.to_bytes();
    let mut rows = connection
        .query(
            "SELECT layout_json, updated_at FROM column_layouts
             WHERE profile_id = ?1 AND database_name = ?2
               AND schema_name = ?3 AND table_name = ?4",
            (
                id.as_slice(),
                key.database.as_str(),
                key.schema.as_str(),
                key.table.as_str(),
            ),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    let Some(row) = rows.next().await.map_err(|_| PersistenceError::Query)? else {
        return Ok(None);
    };
    let layout_json = row.get::<String>(0).map_err(|_| PersistenceError::Query)?;
    let updated_at = row.get::<String>(1).map_err(|_| PersistenceError::Query)?;
    Ok(Some(ColumnLayoutRecord {
        key: key.clone(),
        layout_json,
        updated_at,
    }))
}

pub async fn delete(
    connection: &turso::Connection,
    key: &ColumnLayoutKey,
) -> Result<(), PersistenceError> {
    let id = key.profile_id.to_bytes();
    connection
        .execute(
            "DELETE FROM column_layouts
             WHERE profile_id = ?1 AND database_name = ?2
               AND schema_name = ?3 AND table_name = ?4",
            (
                id.as_slice(),
                key.database.as_str(),
                key.schema.as_str(),
                key.table.as_str(),
            ),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    Ok(())
}
