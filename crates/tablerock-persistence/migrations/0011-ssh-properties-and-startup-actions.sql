-- Expand property ordinals for SSH fields (11–16) and add startup actions.
-- SQLite cannot ALTER CHECK constraints; rebuild the properties table.

CREATE TABLE saved_profile_properties_v2 (
    profile_id BLOB NOT NULL REFERENCES saved_profiles(profile_id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 0 AND 15),
    property INTEGER NOT NULL CHECK(property BETWEEN 1 AND 16),
    source_kind INTEGER NOT NULL CHECK(source_kind BETWEEN 1 AND 6),
    source_schema INTEGER CHECK(source_schema IS NULL OR source_schema = 1),
    text_value TEXT,
    blob_value BLOB,
    op_account_id TEXT,
    op_vault_id TEXT,
    op_item_id TEXT,
    op_section_id TEXT,
    op_field_id TEXT,
    op_breadcrumb TEXT,
    CHECK(
        (source_kind = 1 AND source_schema IS NULL)
        OR (source_kind BETWEEN 2 AND 6 AND source_schema = 1)
    ),
    PRIMARY KEY(profile_id, ordinal),
    UNIQUE(profile_id, property)
);

INSERT INTO saved_profile_properties_v2
SELECT
    profile_id,
    ordinal,
    property,
    source_kind,
    source_schema,
    text_value,
    blob_value,
    op_account_id,
    op_vault_id,
    op_item_id,
    op_section_id,
    op_field_id,
    op_breadcrumb
FROM saved_profile_properties;

DROP TABLE saved_profile_properties;
ALTER TABLE saved_profile_properties_v2 RENAME TO saved_profile_properties;

CREATE TABLE saved_profile_startup_actions (
    profile_id BLOB NOT NULL REFERENCES saved_profiles(profile_id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 0 AND 15),
    statement TEXT NOT NULL CHECK(length(CAST(statement AS BLOB)) BETWEEN 1 AND 8192),
    safety INTEGER NOT NULL CHECK(safety BETWEEN 1 AND 3),
    timeout_ms INTEGER NOT NULL CHECK(timeout_ms BETWEEN 100 AND 120000),
    run_on_reconnect INTEGER NOT NULL CHECK(run_on_reconnect IN (0, 1)),
    PRIMARY KEY(profile_id, ordinal)
);
