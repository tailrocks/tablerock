//! Clipboard-neutral formatters over resident grid cells.
//!
//! Pure functions: no I/O. SQL INSERT/UPDATE require base-table identity.

use super::grid::{DataGridModel, ProjectedCell};
pub use tablerock_core::CopyFormat;
use tablerock_core::{CopyCell, CopyProjectionError, CopyTable, format_copy_table};

/// Copy payload scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyScope {
    Cell,
    Row,
    /// All resident rows currently loaded in the model.
    LoadedResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyError {
    Empty,
    /// INSERT/UPDATE need base_schema + base_table from browse identity.
    MissingTableIdentity,
    MissingStableIdentity,
    BoundsExceeded,
    ShapeMismatch,
}

impl std::fmt::Display for CopyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Empty => "nothing to copy",
            Self::MissingTableIdentity => "SQL INSERT/UPDATE require base-table identity",
            Self::MissingStableIdentity => "SQL UPDATE requires stable identity columns",
            Self::BoundsExceeded => "copy payload exceeds bounds",
            Self::ShapeMismatch => "copy row shape does not match columns",
        })
    }
}

impl From<CopyProjectionError> for CopyError {
    fn from(error: CopyProjectionError) -> Self {
        match error {
            CopyProjectionError::Empty => Self::Empty,
            CopyProjectionError::ShapeMismatch => Self::ShapeMismatch,
            CopyProjectionError::BoundsExceeded => Self::BoundsExceeded,
            CopyProjectionError::MissingTableIdentity => Self::MissingTableIdentity,
            CopyProjectionError::MissingStableIdentity => Self::MissingStableIdentity,
        }
    }
}

/// Copy the cursor cell only (raw presentation text, NULL → empty).
pub fn format_cursor_cell(grid: &DataGridModel) -> Result<String, CopyError> {
    if grid.columns.is_empty() {
        return Err(CopyError::Empty);
    }
    let col = grid.cursor_col.min(grid.columns.len().saturating_sub(1));
    let cell = grid.cell_at(grid.cursor_row, col);
    if matches!(cell.distinction, super::grid::CellDistinction::Pending) {
        return Err(CopyError::Empty);
    }
    if matches!(cell.distinction, super::grid::CellDistinction::Null) {
        return Ok(String::new());
    }
    Ok(cell.text.clone())
}

/// Copy cursor cell as lowercase hex of UTF-8 bytes (binary-friendly).
pub fn format_cursor_cell_hex(grid: &DataGridModel) -> Result<String, CopyError> {
    let text = format_cursor_cell(grid)?;
    if text.is_empty() {
        return Ok(String::new());
    }
    Ok(text
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(""))
}

/// Copy cursor cell as a SQL literal (NULL / number / boolean / quoted text).
pub fn format_cursor_cell_sql(grid: &DataGridModel) -> Result<String, CopyError> {
    use super::grid::CellDistinction;
    if grid.columns.is_empty() {
        return Err(CopyError::Empty);
    }
    let col = grid.cursor_col.min(grid.columns.len().saturating_sub(1));
    let cell = grid.cell_at(grid.cursor_row, col);
    if matches!(cell.distinction, CellDistinction::Pending) {
        return Err(CopyError::Empty);
    }
    Ok(match cell.distinction {
        CellDistinction::Null => "NULL".into(),
        CellDistinction::Boolean => {
            let t = cell.text.trim();
            if t.eq_ignore_ascii_case("true") || t == "t" || t == "1" {
                "TRUE".into()
            } else if t.eq_ignore_ascii_case("false") || t == "f" || t == "0" {
                "FALSE".into()
            } else {
                sql_literal(&cell.text)
            }
        }
        CellDistinction::Number => {
            let t = cell.text.trim();
            // Unquoted only for plain numeric tokens; else quote (e.g. NaN text).
            if t.parse::<f64>().is_ok() && !t.is_empty() {
                t.to_owned()
            } else {
                sql_literal(&cell.text)
            }
        }
        _ => {
            if cell.text.is_empty() {
                "''".into()
            } else {
                sql_literal(&cell.text)
            }
        }
    })
}

/// Presentation aid: `INSERT INTO "schema"."table" ("c1", …) VALUES`.
///
/// Identity-gated. Visible columns only. Does not execute.
pub fn format_insert_sql(grid: &DataGridModel) -> Result<String, CopyError> {
    use super::structure_ddl::quote_ident_sql;
    let (Some(schema), Some(table)) = (grid.base_schema.as_ref(), grid.base_table.as_ref()) else {
        return Err(CopyError::MissingTableIdentity);
    };
    if schema.is_empty() || table.is_empty() {
        return Err(CopyError::MissingTableIdentity);
    }
    let cols = grid.visible_columns();
    if cols.is_empty() {
        return Err(CopyError::Empty);
    }
    let col_list = cols
        .iter()
        .map(|c| quote_ident_sql(c))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!(
        "INSERT INTO {}.{} ({}) VALUES",
        quote_ident_sql(schema),
        quote_ident_sql(table),
        col_list
    ))
}

/// Presentation aid: full `INSERT INTO … VALUES (…)` for the cursor row.
///
/// Identity-gated. Combines [`format_insert_sql`] and [`format_values_sql`].
pub fn format_insert_row_sql(grid: &DataGridModel) -> Result<String, CopyError> {
    let head = format_insert_sql(grid)?;
    let vals = format_values_sql(grid)?;
    Ok(format!("{head} {vals}"))
}

/// Presentation aid: `INSERT INTO … VALUES (r0), (r1), …` for all resident rows.
///
/// Identity-gated. Caps at 500 rows to keep clipboard bounds sane. Does not execute.
pub fn format_insert_loaded_sql(grid: &DataGridModel) -> Result<String, CopyError> {
    let head = format_insert_sql(grid)?;
    let cols = grid.visible_columns();
    if cols.is_empty() || grid.row_count == 0 {
        return Err(CopyError::Empty);
    }
    let n = (grid.row_count as usize).min(500);
    let mut tuples = Vec::with_capacity(n);
    for i in 0..n {
        let abs = grid.start_row.saturating_add(i as u64);
        let mut lits = Vec::with_capacity(cols.len());
        for name in &cols {
            let Some(phys) = grid.columns.iter().position(|c| c == name) else {
                return Err(CopyError::Empty);
            };
            let cell = grid.cell_at(abs, phys);
            if matches!(cell.distinction, super::grid::CellDistinction::Pending) {
                return Err(CopyError::Empty);
            }
            lits.push(cell_sql_literal(&cell));
        }
        tuples.push(format!("({})", lits.join(", ")));
    }
    Ok(format!("{head} {}", tuples.join(",\n")))
}

fn cell_sql_literal(cell: &ProjectedCell) -> String {
    use super::grid::CellDistinction;
    match cell.distinction {
        CellDistinction::Null => "NULL".into(),
        CellDistinction::Boolean => {
            let t = cell.text.trim();
            if t.eq_ignore_ascii_case("true") || t == "t" || t == "1" {
                "TRUE".into()
            } else if t.eq_ignore_ascii_case("false") || t == "f" || t == "0" {
                "FALSE".into()
            } else {
                sql_literal(&cell.text)
            }
        }
        CellDistinction::Number => {
            let t = cell.text.trim();
            if t.parse::<f64>().is_ok() && !t.is_empty() {
                t.to_owned()
            } else {
                sql_literal(&cell.text)
            }
        }
        _ => {
            if cell.text.is_empty() {
                "''".into()
            } else {
                sql_literal(&cell.text)
            }
        }
    }
}

/// Presentation aid: `(lit1, lit2, …)` for the cursor row (visible columns).
///
/// Fails closed when any visible cell is Pending. Does not execute.
pub fn format_values_sql(grid: &DataGridModel) -> Result<String, CopyError> {
    use super::grid::CellDistinction;
    let cols = grid.visible_columns();
    if cols.is_empty() || grid.row_count == 0 {
        return Err(CopyError::Empty);
    }
    let mut lits = Vec::with_capacity(cols.len());
    for name in &cols {
        let Some(phys) = grid.columns.iter().position(|c| c == name) else {
            return Err(CopyError::Empty);
        };
        let cell = grid.cell_at(grid.cursor_row, phys);
        if matches!(cell.distinction, CellDistinction::Pending) {
            return Err(CopyError::Empty);
        }
        lits.push(cell_sql_literal(&cell));
    }
    Ok(format!("({})", lits.join(", ")))
}

/// Copy the cursor column for all resident rows (one value per line).
pub fn format_cursor_column(grid: &DataGridModel) -> Result<String, CopyError> {
    if grid.columns.is_empty() || grid.row_count == 0 {
        return Err(CopyError::Empty);
    }
    let col = grid.cursor_col.min(grid.columns.len().saturating_sub(1));
    let mut lines = Vec::with_capacity(grid.row_count as usize);
    for local in 0..grid.row_count {
        let abs = grid.start_row.saturating_add(u64::from(local));
        let cell = grid.cell_at(abs, col);
        if matches!(cell.distinction, super::grid::CellDistinction::Pending) {
            continue;
        }
        lines.push(cell_plain_text(&cell));
    }
    if lines.is_empty() {
        return Err(CopyError::Empty);
    }
    Ok(lines.join("\n"))
}

/// Format selection from the grid. Returns UTF-8 text for the clipboard effect.
pub fn format_copy(
    grid: &DataGridModel,
    scope: CopyScope,
    format: CopyFormat,
) -> Result<String, CopyError> {
    let visible_columns = if grid.column_layout.is_empty() {
        grid.columns.clone()
    } else {
        grid.visible_columns()
    };
    let columns = if scope == CopyScope::Cell {
        vec![grid.columns[grid.cursor_col.min(grid.columns.len().saturating_sub(1))].clone()]
    } else {
        visible_columns
    };
    if columns.is_empty() {
        return Err(CopyError::Empty);
    }
    let table = CopyTable {
        columns,
        rows: collect_rows(grid, scope),
        base_schema: grid.base_schema.clone(),
        base_table: grid.base_table.clone(),
        identity_columns: grid.identity_columns.clone(),
    };
    format_copy_table(&table, format).map_err(Into::into)
}

fn collect_rows(grid: &DataGridModel, scope: CopyScope) -> Vec<Vec<CopyCell>> {
    let col_count = grid.columns.len();
    if col_count == 0 {
        return Vec::new();
    }
    let abs_rows: Vec<u64> = match scope {
        CopyScope::Cell => vec![grid.cursor_row],
        CopyScope::Row => vec![grid.cursor_row],
        CopyScope::LoadedResult => {
            (grid.start_row..grid.start_row.saturating_add(u64::from(grid.row_count))).collect()
        }
    };
    let visible = if grid.column_layout.is_empty() {
        (0..col_count).collect::<Vec<_>>()
    } else {
        grid.column_layout
            .iter()
            .filter(|c| c.visible)
            .filter_map(|c| grid.columns.iter().position(|n| n == &c.name))
            .collect()
    };
    abs_rows
        .into_iter()
        .map(|abs| match scope {
            CopyScope::Cell => {
                let col = grid.cursor_col.min(col_count.saturating_sub(1));
                vec![copy_cell(&grid.cell_at(abs, col))]
            }
            _ => visible
                .iter()
                .map(|&col| copy_cell(&grid.cell_at(abs, col)))
                .collect(),
        })
        .collect()
}

fn copy_cell(cell: &ProjectedCell) -> CopyCell {
    use super::grid::CellDistinction;
    match cell.distinction {
        CellDistinction::Null => CopyCell::Null,
        CellDistinction::Boolean => CopyCell::Boolean(matches!(
            cell.text.trim().to_ascii_lowercase().as_str(),
            "true" | "t" | "1"
        )),
        CellDistinction::Number => CopyCell::Number(cell.text.clone()),
        CellDistinction::Binary => CopyCell::Binary(cell.text.as_bytes().to_vec()),
        CellDistinction::Unknown => CopyCell::Unknown(cell.text.as_bytes().to_vec()),
        CellDistinction::Invalid => CopyCell::Invalid(cell.text.as_bytes().to_vec()),
        CellDistinction::Truncated => CopyCell::Truncated {
            display: cell.text.clone(),
            original_bytes: cell.original_byte_len,
        },
        CellDistinction::Pending => CopyCell::Text(String::new()),
        _ => CopyCell::Text(cell.text.clone()),
    }
}

fn cell_plain_text(cell: &ProjectedCell) -> String {
    match copy_cell(cell) {
        CopyCell::Null => "NULL".into(),
        CopyCell::Boolean(value) => value.to_string(),
        CopyCell::Number(value) | CopyCell::Text(value) => value,
        CopyCell::Binary(bytes) => format!(
            "\\x{}",
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        CopyCell::Unknown(bytes) => format!("<unknown:{} bytes>", bytes.len()),
        CopyCell::Invalid(bytes) => format!("<invalid:{} bytes>", bytes.len()),
        CopyCell::Truncated { display, .. } => format!("{display}…"),
    }
}

fn sql_literal(value: &str) -> String {
    if value == "NULL" {
        "NULL".into()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

#[cfg(test)]
#[allow(
    clippy::field_reassign_with_default,
    reason = "tests mutate individual options to isolate formatting contracts"
)]
mod tests {
    use super::*;
    use crate::model::grid::{CellDistinction, ProjectedCell};

    fn sample_grid() -> DataGridModel {
        let mut g = DataGridModel::default();
        g.columns = vec!["id".into(), "name".into()];
        g.row_count = 2;
        g.start_row = 0;
        g.cells = vec![
            ProjectedCell {
                text: "1".into(),
                distinction: CellDistinction::Number,
                byte_len: 1,
                original_byte_len: None,
            },
            ProjectedCell {
                text: "a,b".into(),
                distinction: CellDistinction::Text,
                byte_len: 3,
                original_byte_len: None,
            },
            ProjectedCell {
                text: String::new(),
                distinction: CellDistinction::Null,
                byte_len: 0,
                original_byte_len: None,
            },
            ProjectedCell {
                text: "x".into(),
                distinction: CellDistinction::Text,
                byte_len: 1,
                original_byte_len: None,
            },
        ];
        g
    }

    #[test]
    fn csv_quotes_commas_and_null() {
        let g = sample_grid();
        let csv = format_copy(&g, CopyScope::LoadedResult, CopyFormat::Csv).unwrap();
        assert!(csv.contains("id,name"));
        assert!(csv.contains("\"a,b\""));
        assert!(csv.contains("NULL"));
    }

    #[test]
    fn format_cursor_column_resident_values() {
        let mut g = sample_grid();
        g.cursor_col = 0; // id column: 1 and NULL
        let col = format_cursor_column(&g).unwrap();
        let lines: Vec<_> = col.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "1");
        assert_eq!(lines[1], "NULL");
        g.cursor_col = 1;
        let names = format_cursor_column(&g).unwrap();
        assert!(names.contains("a,b") || names.contains("x"), "{names}");
    }

    #[test]
    fn format_cursor_cell_and_hex() {
        let mut g = sample_grid();
        g.cursor_row = 0;
        g.cursor_col = 1; // "a,b"
        assert_eq!(format_cursor_cell(&g).unwrap(), "a,b");
        assert_eq!(format_cursor_cell_hex(&g).unwrap(), "612c62"); // a,b
        g.cursor_col = 0;
        g.cursor_row = 1; // null name? row1 col0 is empty null for id? sample: row1 is null, "x"
        // cells: [1, a,b], [null, x] — row 1 col 0 is null
        g.cursor_row = 1;
        g.cursor_col = 0;
        assert_eq!(format_cursor_cell(&g).unwrap(), "");
        g.cursor_col = 1;
        assert_eq!(format_cursor_cell(&g).unwrap(), "x");
        assert_eq!(format_cursor_cell_hex(&g).unwrap(), "78");
    }

    #[test]
    fn format_cursor_cell_sql_literals() {
        let mut g = sample_grid();
        g.cursor_row = 0;
        g.cursor_col = 0; // number 1
        assert_eq!(format_cursor_cell_sql(&g).unwrap(), "1");
        g.cursor_col = 1; // text a,b
        assert_eq!(format_cursor_cell_sql(&g).unwrap(), "'a,b'");
        g.cursor_row = 1;
        g.cursor_col = 0; // null
        assert_eq!(format_cursor_cell_sql(&g).unwrap(), "NULL");
        g.cells[1] = ProjectedCell {
            text: "o'brien".into(),
            distinction: CellDistinction::Text,
            byte_len: 7,
            original_byte_len: None,
        };
        g.cursor_row = 0;
        g.cursor_col = 1;
        assert_eq!(format_cursor_cell_sql(&g).unwrap(), "'o''brien'");
        g.cells[0] = ProjectedCell {
            text: "true".into(),
            distinction: CellDistinction::Boolean,
            byte_len: 4,
            original_byte_len: None,
        };
        g.cursor_col = 0;
        assert_eq!(format_cursor_cell_sql(&g).unwrap(), "TRUE");
    }

    #[test]
    fn format_insert_and_values_sql() {
        let mut g = sample_grid();
        g.base_schema = Some("public".into());
        g.base_table = Some("users".into());
        g.cursor_row = 0;
        assert_eq!(
            format_insert_sql(&g).unwrap(),
            r#"INSERT INTO "public"."users" ("id", "name") VALUES"#
        );
        assert_eq!(format_values_sql(&g).unwrap(), "(1, 'a,b')");
        assert_eq!(
            format_insert_row_sql(&g).unwrap(),
            r#"INSERT INTO "public"."users" ("id", "name") VALUES (1, 'a,b')"#
        );
        g.base_table = None;
        assert!(matches!(
            format_insert_sql(&g),
            Err(CopyError::MissingTableIdentity)
        ));
        assert!(matches!(
            format_insert_row_sql(&g),
            Err(CopyError::MissingTableIdentity)
        ));
        // Values still works without identity.
        g.base_table = Some("users".into());
        g.cursor_row = 1;
        assert_eq!(format_values_sql(&g).unwrap(), "(NULL, 'x')");
        g.cursor_row = 0;
        let loaded = format_insert_loaded_sql(&g).unwrap();
        assert!(
            loaded.starts_with(r#"INSERT INTO "public"."users""#),
            "{loaded}"
        );
        assert!(loaded.contains("(1, 'a,b')"), "{loaded}");
        assert!(loaded.contains("(NULL, 'x')"), "{loaded}");
    }

    #[test]
    fn format_cursor_row_tsv() {
        let mut g = sample_grid();
        g.cursor_row = 0;
        let tsv = format_copy(&g, CopyScope::Row, CopyFormat::Tsv).unwrap();
        assert!(tsv.contains("1"));
        assert!(tsv.contains("a,b") || tsv.contains("\"a,b\"") || tsv.contains("a,b"));
        // Row scope is single data line (may include header depending on formatter).
        assert!(!tsv.trim().is_empty());
    }

    #[test]
    fn format_cursor_row_csv_json_markdown() {
        let mut g = sample_grid();
        g.cursor_row = 0;
        let csv = format_copy(&g, CopyScope::Row, CopyFormat::Csv).unwrap();
        assert!(csv.contains("id") || csv.contains("1"));
        assert!(csv.contains("a,b") || csv.contains("\"a,b\""));
        let json = format_copy(&g, CopyScope::Row, CopyFormat::Json).unwrap();
        assert!(json.contains("\"id\"") || json.contains("1"));
        let md = format_copy(&g, CopyScope::Row, CopyFormat::Markdown).unwrap();
        assert!(md.contains("|") || md.contains("id"));
        assert!(!csv.is_empty() && !json.is_empty() && !md.is_empty());
    }

    #[test]
    fn format_cursor_row_sql_insert_update() {
        let mut g = sample_grid();
        g.cursor_row = 0;
        assert_eq!(
            format_copy(&g, CopyScope::Row, CopyFormat::SqlInsert),
            Err(CopyError::MissingTableIdentity)
        );
        g.base_schema = Some("public".into());
        g.base_table = Some("users".into());
        let ins = format_copy(&g, CopyScope::Row, CopyFormat::SqlInsert).unwrap();
        assert!(ins.contains("INSERT"), "{ins}");
        assert!(ins.contains("users"), "{ins}");
        assert_eq!(
            format_copy(&g, CopyScope::Row, CopyFormat::SqlUpdate),
            Err(CopyError::MissingStableIdentity)
        );
        g.identity_columns = vec!["id".into()];
        let upd_id = format_copy(&g, CopyScope::Row, CopyFormat::SqlUpdate).unwrap();
        assert!(upd_id.contains("WHERE"), "{upd_id}");
        assert!(upd_id.contains("\"id\""), "{upd_id}");
        assert!(!upd_id.contains("WHERE requires"), "{upd_id}");
    }

    #[test]
    fn sql_insert_requires_identity() {
        let g = sample_grid();
        assert_eq!(
            format_copy(&g, CopyScope::LoadedResult, CopyFormat::SqlInsert),
            Err(CopyError::MissingTableIdentity)
        );
        let mut g = g;
        g.base_schema = Some("public".into());
        g.base_table = Some("users".into());
        let sql = format_copy(&g, CopyScope::LoadedResult, CopyFormat::SqlInsert).unwrap();
        assert!(sql.contains("INSERT INTO \"public\".\"users\""));
        assert!(sql.contains("VALUES (1, 'a,b')") || sql.contains("'a,b'"));
    }

    #[test]
    fn tsv_and_markdown_and_json() {
        let g = sample_grid();
        let tsv = format_copy(&g, CopyScope::LoadedResult, CopyFormat::Tsv).unwrap();
        assert!(tsv.contains("id\tname"));
        let md = format_copy(&g, CopyScope::LoadedResult, CopyFormat::Markdown).unwrap();
        assert!(md.contains("| id |"));
        let json = format_copy(&g, CopyScope::LoadedResult, CopyFormat::Json).unwrap();
        assert!(json.contains("\"id\""));
    }
}
