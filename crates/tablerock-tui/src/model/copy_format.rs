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
            Ok(format_sql_update(schema, table, &columns, &rows))
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

fn format_sql_update(schema: &str, table: &str, columns: &[String], rows: &[Vec<String>]) -> String {
    // Without PK facts we emit SET for all columns; WHERE is left as a comment
    // requiring identity — still gated by base table presence.
    let mut out = String::new();
    for row in rows {
        let sets: Vec<_> = columns
            .iter()
            .zip(row.iter())
            .map(|(c, v)| {
                format!(
                    "\"{}\" = {}",
                    c.replace('"', "\"\""),
                    sql_literal(v)
                )
            })
            .collect();
        out.push_str(&format!(
            "UPDATE \"{}\".\"{}\" SET {}; -- WHERE requires primary key\n",
            schema.replace('"', "\"\""),
            table.replace('"', "\"\""),
            sets.join(", ")
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
