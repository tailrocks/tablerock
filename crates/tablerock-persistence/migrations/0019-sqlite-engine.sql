-- Expand engine CHECK ranges to include SQLite (code 4).

PRAGMA foreign_keys = OFF;

CREATE TABLE saved_profiles_v19 (
    profile_id BLOB NOT NULL PRIMARY KEY CHECK(length(profile_id) = 16),
    aggregate_schema INTEGER NOT NULL CHECK(aggregate_schema = 1),
    connection_schema INTEGER NOT NULL CHECK(connection_schema = 1),
    property_schema INTEGER NOT NULL CHECK(property_schema = 1),
    revision BLOB NOT NULL CHECK(length(revision) = 8),
    engine INTEGER NOT NULL CHECK(engine BETWEEN 1 AND 4),
    name TEXT NOT NULL CHECK(length(CAST(name AS BLOB)) BETWEEN 1 AND 128),
    tls_policy INTEGER NOT NULL CHECK(tls_policy BETWEEN 1 AND 4),
    safety_mode INTEGER NOT NULL CHECK(safety_mode BETWEEN 1 AND 2),
    connect_timeout_ms INTEGER NOT NULL CHECK(connect_timeout_ms BETWEEN 1 AND 120000),
    operation_timeout_ms INTEGER NOT NULL CHECK(operation_timeout_ms BETWEEN 1 AND 3600000),
    max_result_rows INTEGER NOT NULL CHECK(max_result_rows BETWEEN 1 AND 1000000),
    max_result_bytes INTEGER NOT NULL CHECK(max_result_bytes BETWEEN 1 AND 1073741824),
    group_name TEXT CHECK(group_name IS NULL OR length(CAST(group_name AS BLOB)) BETWEEN 1 AND 128),
    favorite INTEGER NOT NULL CHECK(favorite IN (0, 1)),
    saved_order INTEGER NOT NULL CHECK(saved_order BETWEEN 0 AND 4294967295),
    reconnect INTEGER NOT NULL CHECK(reconnect BETWEEN 1 AND 2),
    restore_last_context INTEGER NOT NULL CHECK(restore_last_context IN (0, 1)),
    preferred_page_rows INTEGER NOT NULL CHECK(preferred_page_rows BETWEEN 1 AND 500),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    environment_kind INTEGER CHECK(environment_kind IS NULL OR environment_kind BETWEEN 1 AND 5),
    environment_label TEXT CHECK(environment_label IS NULL OR length(CAST(environment_label AS BLOB)) BETWEEN 1 AND 64),
    ssh_use_agent INTEGER NOT NULL DEFAULT 0 CHECK(ssh_use_agent IN (0, 1))
);

INSERT INTO saved_profiles_v19 SELECT * FROM saved_profiles;
DROP TABLE saved_profiles;
ALTER TABLE saved_profiles_v19 RENAME TO saved_profiles;

CREATE INDEX IF NOT EXISTS saved_profiles_engine_bounded_list
    ON saved_profiles(engine, favorite DESC, saved_order, profile_id);

CREATE TABLE query_history_v19 (
    history_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    engine INTEGER NOT NULL CHECK(engine BETWEEN 1 AND 4),
    database_name TEXT NOT NULL CHECK(length(CAST(database_name AS BLOB)) BETWEEN 0 AND 256),
    schema_name TEXT CHECK(schema_name IS NULL OR length(CAST(schema_name AS BLOB)) BETWEEN 1 AND 256),
    statement_text TEXT CHECK(statement_text IS NULL OR length(CAST(statement_text AS BLOB)) BETWEEN 1 AND 1048576),
    outcome_class TEXT NOT NULL CHECK(outcome_class IN ('completed', 'cancelled', 'failed', 'disconnected', 'unknown')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO query_history_v19 (
    history_id, engine, database_name, schema_name, statement_text, outcome_class, created_at
)
SELECT history_id, engine, database_name, schema_name, statement_text, outcome_class, created_at
FROM query_history;

DROP TABLE query_history;
ALTER TABLE query_history_v19 RENAME TO query_history;
CREATE INDEX IF NOT EXISTS query_history_engine_idx ON query_history(engine, created_at DESC);

CREATE TABLE saved_queries_v19 (
    query_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL CHECK(length(CAST(name AS BLOB)) BETWEEN 1 AND 128),
    engine INTEGER NOT NULL CHECK(engine BETWEEN 1 AND 4),
    statement_text TEXT NOT NULL CHECK(length(CAST(statement_text AS BLOB)) BETWEEN 1 AND 1048576),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(name, engine)
);

INSERT INTO saved_queries_v19 (
    query_id, name, engine, statement_text, created_at, updated_at
)
SELECT query_id, name, engine, statement_text, created_at, updated_at FROM saved_queries;

DROP TABLE saved_queries;
ALTER TABLE saved_queries_v19 RENAME TO saved_queries;
CREATE INDEX IF NOT EXISTS saved_queries_engine_name_idx ON saved_queries(engine, name);

PRAGMA foreign_keys = ON;
