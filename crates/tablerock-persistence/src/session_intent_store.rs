//! Intent-only session restoration per profile (never result payloads).

use tablerock_core::ProfileId;

use crate::PersistenceError;

/// Opaque JSON intent document. Schema is owned by the CLI/TUI bridge:
/// `{ "database": "...", "schema": null|"...", "selected_tab": 0,
///    "tabs": [{ "title": "...", "sql": null|"..." }] }`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionIntentRecord {
    pub profile_id: ProfileId,
    pub intent_json: String,
    pub updated_at: String,
}

pub async fn put(
    connection: &turso::Connection,
    profile_id: ProfileId,
    intent_json: &str,
) -> Result<(), PersistenceError> {
    if intent_json.len() < 2 || intent_json.len() > 2_097_152 {
        return Err(PersistenceError::Query);
    }
    // Reject accidental result-shaped keys at the store boundary.
    if intent_json.contains("\"cells\"")
        || intent_json.contains("\"result_pages\"")
        || intent_json.contains("\"pending_writes\"")
    {
        return Err(PersistenceError::Query);
    }
    let id = profile_id.to_bytes();
    connection
        .execute(
            "INSERT INTO session_intent(profile_id, intent_json, updated_at)
             VALUES (?1, ?2, CURRENT_TIMESTAMP)
             ON CONFLICT(profile_id) DO UPDATE SET
               intent_json = excluded.intent_json,
               updated_at = CURRENT_TIMESTAMP",
            (id.as_slice(), intent_json),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    Ok(())
}

pub async fn get(
    connection: &turso::Connection,
    profile_id: ProfileId,
) -> Result<Option<SessionIntentRecord>, PersistenceError> {
    let id = profile_id.to_bytes();
    let mut rows = connection
        .query(
            "SELECT profile_id, intent_json, updated_at FROM session_intent WHERE profile_id = ?1",
            (id.as_slice(),),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    let Some(row) = rows.next().await.map_err(|_| PersistenceError::Query)? else {
        return Ok(None);
    };
    let _blob = row.get::<Vec<u8>>(0).map_err(|_| PersistenceError::Query)?;
    let intent_json = row.get::<String>(1).map_err(|_| PersistenceError::Query)?;
    let updated_at = row.get::<String>(2).map_err(|_| PersistenceError::Query)?;
    Ok(Some(SessionIntentRecord {
        profile_id,
        intent_json,
        updated_at,
    }))
}

pub async fn delete(
    connection: &turso::Connection,
    profile_id: ProfileId,
) -> Result<(), PersistenceError> {
    let id = profile_id.to_bytes();
    connection
        .execute(
            "DELETE FROM session_intent WHERE profile_id = ?1",
            (id.as_slice(),),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    Ok(())
}
