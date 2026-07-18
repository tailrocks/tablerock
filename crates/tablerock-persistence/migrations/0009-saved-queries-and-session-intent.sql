-- Named saved queries and intent-only session restoration.
-- Never stores result pages, cell values, or pending writes.

CREATE TABLE saved_queries (
    query_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL CHECK(length(CAST(name AS BLOB)) BETWEEN 1 AND 128),
    engine INTEGER NOT NULL CHECK(engine BETWEEN 1 AND 3),
    statement_text TEXT NOT NULL CHECK(
        length(CAST(statement_text AS BLOB)) BETWEEN 1 AND 1048576
    ),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(name, engine)
);

CREATE INDEX saved_queries_engine_name_idx ON saved_queries(engine, name);

-- One intent blob per profile: open tabs + context text only.
CREATE TABLE session_intent (
    profile_id BLOB PRIMARY KEY NOT NULL CHECK(length(profile_id) = 16),
    -- JSON: { database, schema?, selected_tab, tabs: [{ title, sql? }] }
    intent_json TEXT NOT NULL CHECK(length(CAST(intent_json AS BLOB)) BETWEEN 2 AND 2097152),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
