//! Bundled sample SQLite database path and profile naming (both clients).
//!
//! Workflow inspiration: public “try sample database” onboarding exists in the
//! broader DB-client space. Schema, identifiers, and copy are TableRock-owned.

use std::path::{Path, PathBuf};

/// Stable profile name for the operator-visible sample connection.
pub const SAMPLE_DATABASE_PROFILE_NAME: &str = "Sample Database";

/// Relative directory under the operator data root.
pub const SAMPLE_DATABASE_DIR: &str = "samples";

/// File name of the offline sample SQLite database.
pub const SAMPLE_DATABASE_FILE: &str = "tablerock-sample.db";

/// Resolve `{data_root}/samples/tablerock-sample.db`.
#[must_use]
pub fn sample_sqlite_database_path(data_root: &Path) -> PathBuf {
    data_root
        .join(SAMPLE_DATABASE_DIR)
        .join(SAMPLE_DATABASE_FILE)
}

/// True when a path is the canonical sample file name under a samples/ dir.
#[must_use]
pub fn is_sample_sqlite_path(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()) == Some(SAMPLE_DATABASE_FILE)
}

/// Deterministic sample schema (not Chinook / not third-party dumps).
pub const SAMPLE_SCHEMA_SQL: &str = r#"
PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS artists (
  id INTEGER PRIMARY KEY NOT NULL,
  name TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS albums (
  id INTEGER PRIMARY KEY NOT NULL,
  title TEXT NOT NULL,
  artist_id INTEGER NOT NULL REFERENCES artists(id)
);
CREATE TABLE IF NOT EXISTS tracks (
  id INTEGER PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  album_id INTEGER NOT NULL REFERENCES albums(id),
  milliseconds INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS orders (
  id INTEGER PRIMARY KEY NOT NULL,
  customer TEXT NOT NULL,
  total_cents INTEGER NOT NULL,
  status TEXT NOT NULL
);
INSERT OR IGNORE INTO artists(id, name) VALUES
  (1, 'Northwind Quartet'),
  (2, 'Lake District Trio');
INSERT OR IGNORE INTO albums(id, title, artist_id) VALUES
  (1, 'Harbor Light', 1),
  (2, 'Stone Circle', 2);
INSERT OR IGNORE INTO tracks(id, name, album_id, milliseconds) VALUES
  (1, 'Morning Fog', 1, 214000),
  (2, 'Breakwater', 1, 198500),
  (3, 'Ridge Path', 2, 241000),
  (4, 'Cairn', 2, 176250);
INSERT OR IGNORE INTO orders(id, customer, total_cents, status) VALUES
  (1, 'Ada', 2499, 'paid'),
  (2, 'Lin', 1299, 'pending'),
  (3, 'Sam', 4999, 'paid');
"#;

/// Suggested first query for the sample workbench (read-only demo).
pub const SAMPLE_STARTER_SQL: &str = "SELECT t.name AS track, a.title AS album, ar.name AS artist\n\
     FROM tracks t\n\
     JOIN albums a ON a.id = t.album_id\n\
     JOIN artists ar ON ar.id = a.artist_id\n\
     ORDER BY t.id;";

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn sample_path_is_under_data_root() {
        let path = sample_sqlite_database_path(Path::new(
            "/Users/op/Library/Application Support/TableRock",
        ));
        assert!(path.ends_with("samples/tablerock-sample.db"));
        assert!(is_sample_sqlite_path(&path));
    }

    #[test]
    fn sample_schema_names_core_demo_tables() {
        assert!(SAMPLE_SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS artists"));
        assert!(SAMPLE_SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS tracks"));
        assert!(SAMPLE_STARTER_SQL.contains("FROM tracks"));
    }
}
