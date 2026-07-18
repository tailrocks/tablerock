-- Named filter preset library per profile (JSON only; no cell/credential values).

CREATE TABLE saved_filter_libraries (
    profile_id BLOB NOT NULL CHECK(length(profile_id) = 16),
    -- JSON array of presets: [{name,schema,table,raw_where,filters:[...]}]
    library_json TEXT NOT NULL CHECK(length(CAST(library_json AS BLOB)) BETWEEN 2 AND 65536),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (profile_id)
);
