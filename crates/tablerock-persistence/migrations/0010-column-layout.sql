-- Per-table column layout (show/hide/order/width). Never stores cell values.

CREATE TABLE column_layouts (
    profile_id BLOB NOT NULL CHECK(length(profile_id) = 16),
    database_name TEXT NOT NULL CHECK(length(CAST(database_name AS BLOB)) BETWEEN 0 AND 256),
    schema_name TEXT NOT NULL CHECK(length(CAST(schema_name AS BLOB)) BETWEEN 0 AND 256),
    table_name TEXT NOT NULL CHECK(length(CAST(table_name AS BLOB)) BETWEEN 1 AND 256),
    -- JSON array: [{ "name": "...", "visible": true, "width": 12 }, ...]
    layout_json TEXT NOT NULL CHECK(length(CAST(layout_json AS BLOB)) BETWEEN 2 AND 65536),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (profile_id, database_name, schema_name, table_name)
);
