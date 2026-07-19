-- Independent intent-only restoration for native WindowGroup instances.
-- Result pages, cell values, operation handles, and pending writes are forbidden.

CREATE TABLE native_window_session_intent (
    window_id TEXT PRIMARY KEY NOT NULL CHECK(length(window_id) = 36),
    profile_id BLOB NOT NULL CHECK(length(profile_id) = 16),
    intent_json TEXT NOT NULL CHECK(
        length(CAST(intent_json AS BLOB)) BETWEEN 2 AND 2097152
    ),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(profile_id) REFERENCES saved_profiles(profile_id) ON DELETE CASCADE
);

CREATE INDEX native_window_session_intent_profile_idx
    ON native_window_session_intent(profile_id);
