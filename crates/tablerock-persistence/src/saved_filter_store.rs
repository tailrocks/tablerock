//! Named filter preset library persistence (JSON only; no cell values).

use tablerock_core::ProfileId;

use crate::PersistenceError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedFilterLibraryRecord {
    pub profile_id: ProfileId,
    pub library_json: String,
    pub updated_at: String,
}

/// Reject accidental value/result payloads and oversized libraries.
fn validate_library_json(library_json: &str) -> Result<(), PersistenceError> {
    if library_json.len() < 2 || library_json.len() > 65_536 {
        return Err(PersistenceError::Query);
    }
    let trimmed = library_json.trim_start();
    if !trimmed.starts_with('[') {
        return Err(PersistenceError::Query);
    }
    // Fail closed on shapes that look like result pages / credentials.
    if library_json.contains("\"cells\"")
        || library_json.contains("\"result")
        || library_json.contains("password")
        || library_json.contains("secret")
    {
        return Err(PersistenceError::Query);
    }
    Ok(())
}

pub async fn put(
    connection: &turso::Connection,
    profile_id: ProfileId,
    library_json: &str,
) -> Result<(), PersistenceError> {
    validate_library_json(library_json)?;
    let id = profile_id.to_bytes();
    connection
        .execute(
            "INSERT INTO saved_filter_libraries(profile_id, library_json, updated_at)
             VALUES (?1, ?2, CURRENT_TIMESTAMP)
             ON CONFLICT(profile_id) DO UPDATE SET
               library_json = excluded.library_json,
               updated_at = CURRENT_TIMESTAMP",
            (id.as_slice(), library_json),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    Ok(())
}

pub async fn get(
    connection: &turso::Connection,
    profile_id: ProfileId,
) -> Result<Option<SavedFilterLibraryRecord>, PersistenceError> {
    let id = profile_id.to_bytes();
    let mut rows = connection
        .query(
            "SELECT library_json, updated_at FROM saved_filter_libraries
             WHERE profile_id = ?1",
            (id.as_slice(),),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    let Some(row) = rows.next().await.map_err(|_| PersistenceError::Query)? else {
        return Ok(None);
    };
    let library_json = row.get::<String>(0).map_err(|_| PersistenceError::Query)?;
    let updated_at = row.get::<String>(1).map_err(|_| PersistenceError::Query)?;
    Ok(Some(SavedFilterLibraryRecord {
        profile_id,
        library_json,
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
            "DELETE FROM saved_filter_libraries WHERE profile_id = ?1",
            (id.as_slice(),),
        )
        .await
        .map_err(|_| PersistenceError::Query)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_hostile_and_non_array() {
        assert!(validate_library_json("[]").is_ok());
        assert!(validate_library_json(r#"[{"name":"x"}]"#).is_ok());
        assert!(validate_library_json("").is_err());
        assert!(validate_library_json("{}").is_err());
        assert!(validate_library_json(r#"[{"cells":[]}]"#).is_err());
        assert!(validate_library_json(r#"[{"password":"x"}]"#).is_err());
        assert!(validate_library_json(&format!("[{}]", "x".repeat(70_000))).is_err());
    }
}
