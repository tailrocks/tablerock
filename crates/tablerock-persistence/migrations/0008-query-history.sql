-- Bounded query history: statement text optional by retention policy.
-- Never stores result payloads.

CREATE TABLE query_history (
    history_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    engine INTEGER NOT NULL CHECK(engine BETWEEN 1 AND 3),
    database_name TEXT NOT NULL CHECK(length(CAST(database_name AS BLOB)) BETWEEN 0 AND 256),
    schema_name TEXT CHECK(
        schema_name IS NULL OR length(CAST(schema_name AS BLOB)) BETWEEN 1 AND 256
    ),
    -- NULL when retention is off/private for this entry.
    statement_text TEXT CHECK(
        statement_text IS NULL
        OR length(CAST(statement_text AS BLOB)) BETWEEN 1 AND 1048576
    ),
    outcome_class TEXT NOT NULL CHECK(
        outcome_class IN (
            'completed',
            'cancelled',
            'failed',
            'disconnected',
            'unknown'
        )
    ),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX query_history_created_idx ON query_history(created_at DESC);
CREATE INDEX query_history_engine_idx ON query_history(engine, created_at DESC);
