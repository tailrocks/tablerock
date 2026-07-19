use tablerock_core::ProfileId;

use crate::{PersistenceError, session_intent_store::validate_intent_json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeWindowIntentRecord {
    pub window_id: String,
    pub profile_id: ProfileId,
    pub intent_json: String,
    pub updated_at: String,
}

fn validate_window_id(window_id: &str) -> Result<(), PersistenceError> {
    if window_id.len() != 36
        || !window_id.bytes().enumerate().all(|(index, byte)| {
            matches!(index, 8 | 13 | 18 | 23) && byte == b'-'
                || !matches!(index, 8 | 13 | 18 | 23) && byte.is_ascii_hexdigit()
        })
    {
        return Err(PersistenceError::Query);
    }
    Ok(())
}

pub async fn put(
    connection: &turso::Connection,
    window_id: &str,
    profile_id: ProfileId,
    intent_json: &str,
) -> Result<(), PersistenceError> {
    validate_window_id(window_id)?;
    validate_intent_json(intent_json)?;
    let profile_bytes = profile_id.to_bytes();
    connection
        .execute(
            "INSERT INTO native_window_session_intent(
                 window_id, profile_id, intent_json, updated_at
             ) VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)
             ON CONFLICT(window_id) DO UPDATE SET
               profile_id = excluded.profile_id,
               intent_json = excluded.intent_json,
               updated_at = CURRENT_TIMESTAMP",
            (window_id, profile_bytes.as_slice(), intent_json),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    Ok(())
}

pub async fn get(
    connection: &turso::Connection,
    window_id: &str,
) -> Result<Option<NativeWindowIntentRecord>, PersistenceError> {
    validate_window_id(window_id)?;
    let mut rows = connection
        .query(
            "SELECT profile_id, intent_json, updated_at
             FROM native_window_session_intent WHERE window_id = ?1",
            (window_id,),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    let Some(row) = rows.next().await.map_err(|_| PersistenceError::Query)? else {
        return Ok(None);
    };
    let profile_bytes = row.get::<Vec<u8>>(0).map_err(|_| PersistenceError::Query)?;
    let profile_id = ProfileId::from_bytes(
        <[u8; 16]>::try_from(profile_bytes.as_slice()).map_err(|_| PersistenceError::Query)?,
    )
    .map_err(|_| PersistenceError::Query)?;
    Ok(Some(NativeWindowIntentRecord {
        window_id: window_id.to_owned(),
        profile_id,
        intent_json: row.get::<String>(1).map_err(|_| PersistenceError::Query)?,
        updated_at: row.get::<String>(2).map_err(|_| PersistenceError::Query)?,
    }))
}

pub async fn delete(
    connection: &turso::Connection,
    window_id: &str,
) -> Result<(), PersistenceError> {
    validate_window_id(window_id)?;
    connection
        .execute(
            "DELETE FROM native_window_session_intent WHERE window_id = ?1",
            (window_id,),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    Ok(())
}
