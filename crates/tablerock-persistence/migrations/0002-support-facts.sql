CREATE TABLE support_facts (
    fact_key TEXT PRIMARY KEY NOT NULL,
    fact_value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
