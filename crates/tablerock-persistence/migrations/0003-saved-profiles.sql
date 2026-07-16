CREATE TABLE saved_profiles (
    profile_id BLOB PRIMARY KEY NOT NULL CHECK(length(profile_id) = 16),
    aggregate_schema INTEGER NOT NULL CHECK(aggregate_schema = 1),
    connection_schema INTEGER NOT NULL CHECK(connection_schema = 1),
    property_schema INTEGER NOT NULL CHECK(property_schema = 1),
    revision BLOB NOT NULL CHECK(length(revision) = 8),
    engine INTEGER NOT NULL CHECK(engine BETWEEN 1 AND 3),
    name TEXT NOT NULL CHECK(length(CAST(name AS BLOB)) BETWEEN 1 AND 128),
    tls_policy INTEGER NOT NULL CHECK(tls_policy BETWEEN 1 AND 4),
    safety_mode INTEGER NOT NULL CHECK(safety_mode BETWEEN 1 AND 2),
    connect_timeout_ms INTEGER NOT NULL CHECK(connect_timeout_ms BETWEEN 1 AND 120000),
    operation_timeout_ms INTEGER NOT NULL CHECK(operation_timeout_ms BETWEEN 1 AND 3600000),
    max_result_rows INTEGER NOT NULL CHECK(max_result_rows BETWEEN 1 AND 1000000),
    max_result_bytes INTEGER NOT NULL CHECK(max_result_bytes BETWEEN 1 AND 1073741824),
    group_name TEXT CHECK(
        group_name IS NULL OR length(CAST(group_name AS BLOB)) BETWEEN 1 AND 128
    ),
    favorite INTEGER NOT NULL CHECK(favorite IN (0, 1)),
    saved_order INTEGER NOT NULL CHECK(saved_order BETWEEN 0 AND 4294967295),
    reconnect INTEGER NOT NULL CHECK(reconnect BETWEEN 1 AND 2),
    restore_last_context INTEGER NOT NULL CHECK(restore_last_context IN (0, 1)),
    preferred_page_rows INTEGER NOT NULL CHECK(preferred_page_rows BETWEEN 1 AND 500),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE saved_profile_tags (
    profile_id BLOB NOT NULL REFERENCES saved_profiles(profile_id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 0 AND 31),
    tag TEXT NOT NULL CHECK(length(CAST(tag AS BLOB)) BETWEEN 1 AND 64),
    PRIMARY KEY(profile_id, ordinal),
    UNIQUE(profile_id, tag)
);

CREATE TABLE saved_profile_properties (
    profile_id BLOB NOT NULL REFERENCES saved_profiles(profile_id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 0 AND 9),
    property INTEGER NOT NULL CHECK(property BETWEEN 1 AND 10),
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
    CHECK(
        (source_kind IN (1, 4) AND text_value IS NOT NULL)
        OR (source_kind NOT IN (1, 4) AND text_value IS NULL)
    ),
    CHECK(
        (source_kind = 5 AND length(blob_value) BETWEEN 1 AND 4096)
        OR (source_kind = 6 AND length(blob_value) BETWEEN 1 AND 65536)
        OR (source_kind NOT IN (5, 6) AND blob_value IS NULL)
    ),
    CHECK(
        (source_kind = 2 AND length(op_account_id) = 26 AND length(op_vault_id) = 26
            AND length(op_item_id) = 26
            AND (op_section_id IS NULL OR length(CAST(op_section_id AS BLOB)) BETWEEN 1 AND 128)
            AND length(CAST(op_field_id AS BLOB)) BETWEEN 1 AND 128
            AND length(CAST(op_breadcrumb AS BLOB)) BETWEEN 1 AND 256)
        OR (source_kind != 2 AND op_account_id IS NULL AND op_vault_id IS NULL
            AND op_item_id IS NULL AND op_section_id IS NULL AND op_field_id IS NULL
            AND op_breadcrumb IS NULL)
    ),
    CHECK(source_kind != 4 OR length(CAST(text_value AS BLOB)) BETWEEN 1 AND 128),
    PRIMARY KEY(profile_id, ordinal),
    UNIQUE(profile_id, property)
);

CREATE INDEX saved_profiles_organization
    ON saved_profiles(favorite DESC, saved_order, name, profile_id);
CREATE INDEX saved_profile_tags_lookup
    ON saved_profile_tags(tag, profile_id);
