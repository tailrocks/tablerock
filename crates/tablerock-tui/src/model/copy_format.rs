//! Clipboard-neutral formatters over resident grid cells.
//!
//! Pure functions: no I/O. SQL INSERT/UPDATE require base-table identity.

use super::grid::{DataGridModel, ProjectedCell};

/// Copy payload scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyScope {
    Cell,
    Row,
    /// All resident rows currently loaded in the model.
    LoadedResult,
}

/// Requested output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyFormat {
    Csv,
    Tsv,
    Json,
    Markdown,
    SqlInsert,
    SqlUpdate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyError {
    Empty,
    /// INSERT/UPDATE need base_schema + base_table from browse identity.
    MissingTableIdentity,
}

impl std::fmt::Display for CopyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Empty => "nothing to copy",
            Self::MissingTableIdentity => "SQL INSERT/UPDATE require base-table identity",
        })
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
        let lit = match cell.distinction {
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
        };
        lits.push(lit);
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
        lines.push(cell_copy_text(&cell));
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
    let rows = collect_rows(grid, scope);
    if rows.is_empty() {
        return Err(CopyError::Empty);
    }
    let columns = if grid.column_layout.is_empty() {
        grid.columns.clone()
    } else {
        grid.visible_columns()
    };
    if columns.is_empty() {
        return Err(CopyError::Empty);
    }
    match format {
        CopyFormat::Csv => Ok(format_csv(&columns, &rows)),
        CopyFormat::Tsv => Ok(format_tsv(&columns, &rows)),
        CopyFormat::Json => Ok(format_json(&columns, &rows)),
        CopyFormat::Markdown => Ok(format_markdown(&columns, &rows)),
        CopyFormat::SqlInsert => {
            let (schema, table) = table_identity(grid)?;
            Ok(format_sql_insert(schema, table, &columns, &rows))
        }
        CopyFormat::SqlUpdate => {
            let (schema, table) = table_identity(grid)?;
            Ok(format_sql_update(
                schema,
                table,
                &columns,
                &rows,
                &grid.identity_columns,
            ))
        }
    }
}

fn table_identity(grid: &DataGridModel) -> Result<(&str, &str), CopyError> {
    match (&grid.base_schema, &grid.base_table) {
        (Some(s), Some(t)) if !s.is_empty() && !t.is_empty() => Ok((s.as_str(), t.as_str())),
        _ => Err(CopyError::MissingTableIdentity),
    }
}

fn collect_rows(grid: &DataGridModel, scope: CopyScope) -> Vec<Vec<String>> {
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
        .map(|abs| {
            match scope {
                CopyScope::Cell => {
                    let col = grid.cursor_col.min(col_count.saturating_sub(1));
                    vec![cell_copy_text(&grid.cell_at(abs, col))]
                }
                _ => visible
                    .iter()
                    .map(|&col| cell_copy_text(&grid.cell_at(abs, col)))
                    .collect(),
            }
        })
        .collect()
}

fn cell_copy_text(cell: &ProjectedCell) -> String {
    use super::grid::CellDistinction;
    match cell.distinction {
        CellDistinction::Null => "NULL".into(),
        CellDistinction::Pending => String::new(),
        CellDistinction::Truncated => format!("{}…", cell.text),
        CellDistinction::Binary => {
            if cell.text.is_empty() {
                "\\x".into()
            } else {
                format!("\\x{}", cell.text.replace(' ', ""))
            }
        }
        CellDistinction::Unknown | CellDistinction::Invalid => {
            if cell.text.is_empty() {
                "?".into()
            } else {
                cell.text.clone()
            }
        }
        _ => cell.text.clone(),
    }
}

fn format_csv(columns: &[String], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    out.push_str(&columns.iter().map(|c| csv_escape(c)).collect::<Vec<_>>().join(","));
    out.push('\n');
    for row in rows {
        let line: Vec<_> = row.iter().map(|c| csv_escape(c)).collect();
        out.push_str(&line.join(","));
        out.push('\n');
    }
    out
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r')
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn format_tsv(columns: &[String], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    out.push_str(&columns.join("\t"));
    out.push('\n');
    for row in rows {
        // TSV: no quoting; replace tabs/newlines in cells.
        let line: Vec<_> = row
            .iter()
            .map(|c| c.replace(['\t', '\n', '\r'], " "))
            .collect();
        out.push_str(&line.join("\t"));
        out.push('\n');
    }
    out
}

fn format_json(columns: &[String], rows: &[Vec<String>]) -> String {
    let mut out = String::from("[\n");
    for (ri, row) in rows.iter().enumerate() {
        out.push_str("  {");
        for (ci, col) in columns.iter().enumerate() {
            if ci > 0 {
                out.push_str(", ");
            }
            let val = row.get(ci).map(String::as_str).unwrap_or("");
            out.push_str(&format!(
                "\"{}\":{}",
                json_escape_key(col),
                json_value(val)
            ));
        }
        out.push('}');
        if ri + 1 < rows.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("]\n");
    out
}

fn json_escape_key(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn json_value(s: &str) -> String {
    if s == "NULL" {
        "null".into()
    } else if s == "true" || s == "false" {
        s.to_owned()
    } else if s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok() {
        s.to_owned()
    } else {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

fn format_markdown(columns: &[String], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    out.push('|');
    for c in columns {
        out.push_str(&format!(" {} |", c.replace('|', "\\|")));
    }
    out.push('\n');
    out.push('|');
    for _ in columns {
        out.push_str(" --- |");
    }
    out.push('\n');
    for row in rows {
        out.push('|');
        for cell in row {
            out.push_str(&format!(" {} |", cell.replace('|', "\\|")));
        }
        out.push('\n');
    }
    out
}

fn format_sql_insert(schema: &str, table: &str, columns: &[String], rows: &[Vec<String>]) -> String {
    let cols = columns
        .iter()
        .map(|c| format!("\"{}\"", c.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = String::new();
    for row in rows {
        let vals = row
            .iter()
            .map(|v| sql_literal(v))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "INSERT INTO \"{}\".\"{}\" ({cols}) VALUES ({vals});\n",
            schema.replace('"', "\"\""),
            table.replace('"', "\"\"")
        ));
    }
    out
}

fn format_sql_update(
    schema: &str,
    table: &str,
    columns: &[String],
    rows: &[Vec<String>],
    identity_columns: &[String],
) -> String {
    // Prefer WHERE from proven identity columns; otherwise comment that WHERE
    // needs a primary key (still gated by base table presence).
    let mut out = String::new();
    for row in rows {
        let sets: Vec<_> = columns
            .iter()
            .zip(row.iter())
            .filter(|(c, _)| !identity_columns.iter().any(|id| id == *c))
            .map(|(c, v)| {
                format!(
                    "\"{}\" = {}",
                    c.replace('"', "\"\""),
                    sql_literal(v)
                )
            })
            .collect();
        // If every column is identity, SET all columns so the statement is usable.
        let sets = if sets.is_empty() {
            columns
                .iter()
                .zip(row.iter())
                .map(|(c, v)| {
                    format!(
                        "\"{}\" = {}",
                        c.replace('"', "\"\""),
                        sql_literal(v)
                    )
                })
                .collect::<Vec<_>>()
        } else {
            sets
        };
        let where_clause = if identity_columns.is_empty() {
            "-- WHERE requires primary key".to_owned()
        } else {
            let parts: Vec<String> = identity_columns
                .iter()
                .filter_map(|id| {
                    let idx = columns.iter().position(|c| c == id)?;
                    let val = row.get(idx)?;
                    Some(format!(
                        "\"{}\" = {}",
                        id.replace('"', "\"\""),
                        sql_literal(val)
                    ))
                })
                .collect();
            if parts.is_empty() {
                "-- WHERE requires primary key".to_owned()
            } else {
                format!("WHERE {}", parts.join(" AND "))
            }
        };
        out.push_str(&format!(
            "UPDATE \"{}\".\"{}\" SET {} {};\n",
            schema.replace('"', "\"\""),
            table.replace('"', "\"\""),
            sets.join(", "),
            where_clause
        ));
    }
    out
}

fn sql_literal(value: &str) -> String {
    if value == "NULL" {
        "NULL".into()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

#[cfg(test)]
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
        let upd = format_copy(&g, CopyScope::Row, CopyFormat::SqlUpdate).unwrap();
        assert!(upd.contains("UPDATE"), "{upd}");
        assert!(upd.contains("users"), "{upd}");
        assert!(upd.contains("WHERE requires primary key") || upd.contains("WHERE"), "{upd}");
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
