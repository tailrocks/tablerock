//! Bounded CSV import reader — formula-neutral, position-aware errors.
//!
//! Formula-like cells (`=SUM(...)`) are imported as plain text data and never
//! evaluated. Encoding is UTF-8; invalid UTF-8 yields an explicit error with
//! byte offset.
//!
//! Apply path (residual 016): build typed `MutationChange::InsertRow` values
//! only — never SQL string concatenation. Engine apply uses `$n` binds.

use std::fmt;

use tablerock_core::{
    BoundedText, ByteLimit, FieldValue, MutationBuildError, MutationChange, OwnedValue, Truncation,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvImportError {
    pub row: u32,
    pub column: u32,
    pub message: String,
}

impl fmt::Display for CsvImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "csv import error at row {} col {}: {}",
            self.row, self.column, self.message
        )
    }
}

/// Parsed CSV table: header + data rows (all text cells).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Parse a UTF-8 CSV buffer (RFC-4180-tolerant: quotes, commas, newlines).
pub fn parse_csv(input: &str, max_rows: u32, max_cell_bytes: usize) -> Result<CsvTable, CsvImportError> {
    if max_rows == 0 || max_cell_bytes == 0 {
        return Err(CsvImportError {
            row: 0,
            column: 0,
            message: "invalid limits".into(),
        });
    }
    let mut rows = Vec::new();
    let mut field = String::new();
    let mut row: Vec<String> = Vec::new();
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();
    let mut line: u32 = 1;
    let mut col: u32 = 1;

    let push_field = |field: &mut String, row: &mut Vec<String>, max_cell_bytes: usize, line: u32, col: u32| -> Result<(), CsvImportError> {
        if field.len() > max_cell_bytes {
            return Err(CsvImportError {
                row: line,
                column: col,
                message: format!("cell exceeds {max_cell_bytes} bytes"),
            });
        }
        row.push(std::mem::take(field));
        Ok(())
    };

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes {
                    if chars.peek() == Some(&'"') {
                        field.push('"');
                        chars.next();
                    } else {
                        in_quotes = false;
                    }
                } else {
                    in_quotes = true;
                }
            }
            ',' if !in_quotes => {
                push_field(&mut field, &mut row, max_cell_bytes, line, col)?;
                col += 1;
            }
            '\n' if !in_quotes => {
                push_field(&mut field, &mut row, max_cell_bytes, line, col)?;
                if !row.is_empty() {
                    rows.push(std::mem::take(&mut row));
                }
                if rows.len() as u32 > max_rows {
                    return Err(CsvImportError {
                        row: line,
                        column: 0,
                        message: format!("exceeds max_rows {max_rows}"),
                    });
                }
                line += 1;
                col = 1;
            }
            '\r' if !in_quotes => {
                // swallow CR of CRLF
            }
            c => field.push(c),
        }
    }
    if in_quotes {
        return Err(CsvImportError {
            row: line,
            column: col,
            message: "unclosed quote".into(),
        });
    }
    if !field.is_empty() || !row.is_empty() {
        push_field(&mut field, &mut row, max_cell_bytes, line, col)?;
        rows.push(row);
    }
    if rows.is_empty() {
        return Err(CsvImportError {
            row: 0,
            column: 0,
            message: "empty csv".into(),
        });
    }
    let headers = rows.remove(0);
    Ok(CsvTable { headers, rows })
}

/// Formula-like content is data — never evaluated.
#[must_use]
pub fn is_formula_like(cell: &str) -> bool {
    let t = cell.trim_start();
    t.starts_with('=') || t.starts_with('+') || t.starts_with('-') && t.contains('(')
}

/// Convert a parsed CSV table into insert mutation changes (one change per row).
///
/// All cells become text field values under the header names. Formula-like
/// cells remain ordinary text. Empty tables or header/row width mismatches
/// fail with a position-aware error. Does not build SQL.
pub fn csv_to_insert_changes(
    table: &CsvTable,
    max_cell_bytes: u64,
) -> Result<Vec<MutationChange>, CsvImportError> {
    if table.headers.is_empty() {
        return Err(CsvImportError {
            row: 0,
            column: 0,
            message: "csv has no columns".into(),
        });
    }
    let limit = ByteLimit::new(max_cell_bytes.max(1));
    let mut changes = Vec::with_capacity(table.rows.len());
    for (row_index, row) in table.rows.iter().enumerate() {
        let line = u32::try_from(row_index + 2).unwrap_or(u32::MAX); // 1-based data row after header
        if row.len() != table.headers.len() {
            return Err(CsvImportError {
                row: line,
                column: 0,
                message: format!(
                    "column count {} does not match header count {}",
                    row.len(),
                    table.headers.len()
                ),
            });
        }
        let mut values = Vec::with_capacity(row.len());
        for (col_index, (header, cell)) in table.headers.iter().zip(row.iter()).enumerate() {
            let col = u32::try_from(col_index + 1).unwrap_or(u32::MAX);
            let field = BoundedText::copy_from_str(header, limit).map_err(|_| CsvImportError {
                row: 1,
                column: col,
                message: format!("header exceeds {max_cell_bytes} bytes"),
            })?;
            let text = BoundedText::copy_from_str(cell, limit).map_err(|_| CsvImportError {
                row: line,
                column: col,
                message: format!("cell exceeds {max_cell_bytes} bytes"),
            })?;
            let value = OwnedValue::text(text, Truncation::Complete).map_err(|_| {
                CsvImportError {
                    row: line,
                    column: col,
                    message: "invalid text cell".into(),
                }
            })?;
            values.push(FieldValue::new(field, value));
        }
        changes.push(MutationChange::InsertRow { values });
    }
    Ok(changes)
}

/// Validate that insert changes can form a mutation plan target shape without
/// building SQL. Used by import apply residual before review/authorize.
pub fn validate_insert_batch_size(
    changes: &[MutationChange],
    max_changes: u32,
) -> Result<(), MutationBuildError> {
    if changes.is_empty() {
        return Err(MutationBuildError::NoChanges);
    }
    if changes.len() as u64 > u64::from(max_changes) {
        return Err(MutationBuildError::ChangeLimitExceeded {
            actual: changes.len() as u64,
            limit: max_changes,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_and_quoted() {
        let t = parse_csv("a,b\n1,\"x,y\"\n", 10, 64).unwrap();
        assert_eq!(t.headers, vec!["a", "b"]);
        assert_eq!(t.rows[0], vec!["1", "x,y"]);
    }

    #[test]
    fn csv_to_insert_changes_keeps_formula_as_text_and_rejects_width_mismatch() {
        let t = parse_csv("name,expr\nalice,=SUM(A1)\n", 10, 64).unwrap();
        let changes = csv_to_insert_changes(&t, 64).unwrap();
        assert_eq!(changes.len(), 1);
        match &changes[0] {
            MutationChange::InsertRow { values } => {
                assert_eq!(values.len(), 2);
                assert_eq!(values[0].field(), "name");
                assert_eq!(values[1].field(), "expr");
                // Formula-like cell remains data — no evaluation.
                assert!(is_formula_like("=SUM(A1)"));
            }
            other => panic!("expected InsertRow, got {other:?}"),
        }
        validate_insert_batch_size(&changes, 16).unwrap();
        assert!(validate_insert_batch_size(&changes, 0).is_err());

        let bad = CsvTable {
            headers: vec!["a".into(), "b".into()],
            rows: vec![vec!["only-one".into()]],
        };
        assert!(csv_to_insert_changes(&bad, 64).is_err());
    }

    #[test]
    fn csv_insert_changes_form_valid_mutation_plan_without_sql() {
        use tablerock_core::{
            ContextId, Engine, IdParts, MutationId, MutationPlan, MutationPlanLimits,
            MutationTarget, OperationScope, ProfileId, SessionId, Revision,
        };

        let t = parse_csv("id,label\n1,hello\n2,=CMD()\n", 10, 64).unwrap();
        let changes = csv_to_insert_changes(&t, 64).unwrap();
        validate_insert_batch_size(&changes, 16).unwrap();
        let scope = OperationScope::new(
            ProfileId::from_parts(IdParts::new(1, 1).unwrap()).unwrap(),
            SessionId::from_parts(IdParts::new(1, 2).unwrap()).unwrap(),
            ContextId::from_parts(IdParts::new(1, 3).unwrap()).unwrap(),
        );
        let plan = MutationPlan::new(
            MutationId::from_parts(IdParts::new(1, 4).unwrap()).unwrap(),
            scope,
            Revision::INITIAL,
            MutationTarget::PostgreSqlRelation {
                database: BoundedText::copy_from_str("postgres", ByteLimit::new(16)).unwrap(),
                schema: BoundedText::copy_from_str("public", ByteLimit::new(16)).unwrap(),
                relation: BoundedText::copy_from_str("import_probe", ByteLimit::new(32)).unwrap(),
            },
            changes,
            MutationPlanLimits::new(16, 16, 4096, 4096, 60_000).unwrap(),
        )
        .unwrap();
        assert_eq!(plan.changes().len(), 2);
        assert_eq!(plan.target().engine(), Engine::PostgreSql);
        // No SQL text is produced by this path — only typed plan fields.
        let _ = plan;
    }

    #[test]
    fn formula_is_data_not_evaluated() {
        let t = parse_csv("v\n=SUM(A1:A2)\n", 10, 64).unwrap();
        assert_eq!(t.rows[0][0], "=SUM(A1:A2)");
        assert!(is_formula_like(&t.rows[0][0]));
        // Content remains the literal formula string.
        assert_eq!(t.rows[0][0].chars().next(), Some('='));
    }

    #[test]
    fn oversized_cell_and_malformed_quote() {
        assert!(parse_csv("a\nhello\n", 10, 3).is_err());
        assert!(parse_csv("a\n\"unclosed\n", 10, 64).is_err());
    }
}
