//! Bounded CSV import reader — formula-neutral, position-aware errors.
//!
//! Formula-like cells (`=SUM(...)`) are imported as plain text data and never
//! evaluated. Encoding is UTF-8; invalid UTF-8 yields an explicit error with
//! byte offset.
//!
//! Apply path (residual 016): build typed `MutationChange::InsertRow` values
//! only — never SQL string concatenation. Engine apply uses `$n` binds.

use std::{
    fmt,
    fs::File,
    io::{self, Read},
    path::Path,
};

use tablerock_core::{
    BoundedText, ByteLimit, FieldValue, MutationBuildError, MutationChange, OwnedValue, Truncation,
};

const MAX_CSV_COLUMNS: usize = 1_024;

#[derive(Debug)]
pub enum CsvFileError {
    InvalidLimit,
    TooLarge { actual: u64, limit: u64 },
    InvalidUtf8 { byte_offset: usize },
    Io(io::Error),
    Parse(CsvImportError),
}

impl fmt::Display for CsvFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit => formatter.write_str("csv file limit must be nonzero"),
            Self::TooLarge { actual, limit } => {
                write!(formatter, "csv file has {actual} bytes; limit is {limit}")
            }
            Self::InvalidUtf8 { byte_offset } => {
                write!(formatter, "csv is not UTF-8 at byte {byte_offset}")
            }
            Self::Io(error) => write!(formatter, "csv file I/O: {error}"),
            Self::Parse(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for CsvFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Parse(error) => Some(error),
            _ => None,
        }
    }
}

/// Read and parse one CSV file without ever buffering beyond `max_file_bytes + 1`.
pub fn read_csv_bounded(
    path: &Path,
    max_file_bytes: u64,
    max_rows: u32,
    max_cell_bytes: usize,
) -> Result<CsvTable, CsvFileError> {
    if max_file_bytes == 0 {
        return Err(CsvFileError::InvalidLimit);
    }
    let metadata = path.metadata().map_err(CsvFileError::Io)?;
    if metadata.len() > max_file_bytes {
        return Err(CsvFileError::TooLarge {
            actual: metadata.len(),
            limit: max_file_bytes,
        });
    }
    let capacity = usize::try_from(metadata.len().min(max_file_bytes)).unwrap_or(usize::MAX);
    let mut bytes = Vec::with_capacity(capacity);
    File::open(path)
        .map_err(CsvFileError::Io)?
        .take(max_file_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(CsvFileError::Io)?;
    if bytes.len() as u64 > max_file_bytes {
        return Err(CsvFileError::TooLarge {
            actual: bytes.len() as u64,
            limit: max_file_bytes,
        });
    }
    let text = std::str::from_utf8(&bytes).map_err(|error| CsvFileError::InvalidUtf8 {
        byte_offset: error.valid_up_to(),
    })?;
    parse_csv(text, max_rows, max_cell_bytes).map_err(CsvFileError::Parse)
}

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

impl std::error::Error for CsvImportError {}

/// Parsed CSV table: header + data rows (all text cells).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvValueType {
    Text,
    Signed,
    Float64,
    Boolean,
}

/// Parse a UTF-8 CSV buffer (RFC-4180-tolerant: quotes, commas, newlines).
pub fn parse_csv(
    input: &str,
    max_rows: u32,
    max_cell_bytes: usize,
) -> Result<CsvTable, CsvImportError> {
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
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum FieldState {
        Start,
        Unquoted,
        Quoted,
        AfterQuote,
    }
    let mut state = FieldState::Start;
    let mut chars = input.chars().peekable();
    let mut line: u32 = 1;
    let mut col: u32 = 1;

    let push_field = |field: &mut String,
                      row: &mut Vec<String>,
                      max_cell_bytes: usize,
                      line: u32,
                      col: u32|
     -> Result<(), CsvImportError> {
        if row.len() >= MAX_CSV_COLUMNS {
            return Err(CsvImportError {
                row: line,
                column: col,
                message: format!("exceeds max columns {MAX_CSV_COLUMNS}"),
            });
        }
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
        match (state, ch) {
            (FieldState::Start, '"') => state = FieldState::Quoted,
            (FieldState::Start, ',') => {
                push_field(&mut field, &mut row, max_cell_bytes, line, col)?;
                col += 1;
            }
            (FieldState::Start, '\n') => {
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
            (FieldState::Start, '\r') => {}
            (FieldState::Start, c) => {
                field.push(c);
                state = FieldState::Unquoted;
            }
            (FieldState::Unquoted, '"') => {
                return Err(CsvImportError {
                    row: line,
                    column: col,
                    message: "quote inside unquoted field".into(),
                });
            }
            (FieldState::Unquoted, ',') => {
                push_field(&mut field, &mut row, max_cell_bytes, line, col)?;
                col += 1;
                state = FieldState::Start;
            }
            (FieldState::Unquoted, '\n') => {
                push_field(&mut field, &mut row, max_cell_bytes, line, col)?;
                rows.push(std::mem::take(&mut row));
                if rows.len() as u32 > max_rows {
                    return Err(CsvImportError {
                        row: line,
                        column: 0,
                        message: format!("exceeds max_rows {max_rows}"),
                    });
                }
                line += 1;
                col = 1;
                state = FieldState::Start;
            }
            (FieldState::Unquoted, '\r') => {}
            (FieldState::Unquoted, c) => field.push(c),
            (FieldState::Quoted, '"') if chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            (FieldState::Quoted, '"') => state = FieldState::AfterQuote,
            (FieldState::Quoted, c) => field.push(c),
            (FieldState::AfterQuote, ',') => {
                push_field(&mut field, &mut row, max_cell_bytes, line, col)?;
                col += 1;
                state = FieldState::Start;
            }
            (FieldState::AfterQuote, '\n') => {
                push_field(&mut field, &mut row, max_cell_bytes, line, col)?;
                rows.push(std::mem::take(&mut row));
                if rows.len() as u32 > max_rows {
                    return Err(CsvImportError {
                        row: line,
                        column: 0,
                        message: format!("exceeds max_rows {max_rows}"),
                    });
                }
                line += 1;
                col = 1;
                state = FieldState::Start;
            }
            (FieldState::AfterQuote, '\r') => {}
            (FieldState::AfterQuote, _) => {
                return Err(CsvImportError {
                    row: line,
                    column: col,
                    message: "unexpected content after closing quote".into(),
                });
            }
        }
    }
    if state == FieldState::Quoted {
        return Err(CsvImportError {
            row: line,
            column: col,
            message: "unclosed quote".into(),
        });
    }
    if state != FieldState::Start || !field.is_empty() || !row.is_empty() {
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
    if headers.iter().any(String::is_empty) {
        return Err(CsvImportError {
            row: 1,
            column: 0,
            message: "headers must be non-empty".into(),
        });
    }
    let unique = headers.iter().collect::<std::collections::BTreeSet<_>>();
    if unique.len() != headers.len() {
        return Err(CsvImportError {
            row: 1,
            column: 0,
            message: "headers must be unique".into(),
        });
    }
    Ok(CsvTable { headers, rows })
}

/// Formula-like content is data — never evaluated.
#[must_use]
pub fn is_formula_like(cell: &str) -> bool {
    let t = cell.trim_start();
    t.starts_with(['=', '+', '-', '@'])
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
    csv_to_typed_insert_changes(
        table,
        &vec![CsvValueType::Text; table.headers.len()],
        max_cell_bytes,
    )
}

/// Convert CSV rows with explicit operator-reviewed value types.
pub fn csv_to_typed_insert_changes(
    table: &CsvTable,
    value_types: &[CsvValueType],
    max_cell_bytes: u64,
) -> Result<Vec<MutationChange>, CsvImportError> {
    if table.headers.is_empty() {
        return Err(CsvImportError {
            row: 0,
            column: 0,
            message: "csv has no columns".into(),
        });
    }
    if value_types.len() != table.headers.len() {
        return Err(CsvImportError {
            row: 1,
            column: 0,
            message: "value type count does not match header count".into(),
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
        for (col_index, ((header, cell), value_type)) in table
            .headers
            .iter()
            .zip(row.iter())
            .zip(value_types.iter())
            .enumerate()
        {
            let col = u32::try_from(col_index + 1).unwrap_or(u32::MAX);
            let field = BoundedText::copy_from_str(header, limit).map_err(|_| CsvImportError {
                row: 1,
                column: col,
                message: format!("header exceeds {max_cell_bytes} bytes"),
            })?;
            let invalid = |expected: &str| CsvImportError {
                row: line,
                column: col,
                message: format!("value is not valid {expected}"),
            };
            let value = match value_type {
                CsvValueType::Text => {
                    let text =
                        BoundedText::copy_from_str(cell, limit).map_err(|_| CsvImportError {
                            row: line,
                            column: col,
                            message: format!("cell exceeds {max_cell_bytes} bytes"),
                        })?;
                    OwnedValue::text(text, Truncation::Complete).map_err(|_| CsvImportError {
                        row: line,
                        column: col,
                        message: "invalid text cell".into(),
                    })?
                }
                CsvValueType::Signed => {
                    OwnedValue::signed(cell.parse::<i64>().map_err(|_| invalid("signed integer"))?)
                }
                CsvValueType::Float64 => {
                    let parsed = cell.parse::<f64>().map_err(|_| invalid("finite float"))?;
                    if !parsed.is_finite() {
                        return Err(invalid("finite float"));
                    }
                    OwnedValue::float64_bits(parsed.to_bits())
                }
                CsvValueType::Boolean => {
                    OwnedValue::boolean(match cell.to_ascii_lowercase().as_str() {
                        "true" | "t" | "1" => true,
                        "false" | "f" | "0" => false,
                        _ => return Err(invalid("boolean")),
                    })
                }
            };
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
    fn rejects_ambiguous_quotes_and_invalid_headers() {
        for input in ["a\nplain\"quote\n", "a\n\"closed\"tail\n"] {
            assert!(parse_csv(input, 10, 64).is_err(), "accepted {input:?}");
        }
        assert!(parse_csv("a,a\n1,2\n", 10, 64).is_err());
        assert!(parse_csv(",b\n1,2\n", 10, 64).is_err());
        let empty = parse_csv("a\n\"\"", 10, 64).unwrap();
        assert_eq!(empty.rows, vec![vec![String::new()]]);
    }

    #[test]
    fn bounds_column_count_and_flags_spreadsheet_formulas() {
        let too_wide = format!("{}\n", vec!["h"; MAX_CSV_COLUMNS + 1].join(","));
        assert!(parse_csv(&too_wide, 2, 8).is_err());
        for value in ["=1+1", "+cmd", "-2", "@SUM(A1)", "  =trimmed"] {
            assert!(is_formula_like(value), "missed {value:?}");
        }
        assert!(!is_formula_like("plain"));
    }

    #[test]
    fn bounded_file_read_rejects_size_and_utf8_before_parsing() {
        let dir = std::env::temp_dir().join(format!(
            "tablerock-csv-read-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let valid = dir.join("valid.csv");
        std::fs::write(&valid, b"id,name\n1,Ada\n").unwrap();
        assert_eq!(read_csv_bounded(&valid, 64, 10, 16).unwrap().rows.len(), 1);
        assert!(matches!(
            read_csv_bounded(&valid, 4, 10, 16),
            Err(CsvFileError::TooLarge { .. })
        ));
        let invalid = dir.join("invalid.csv");
        std::fs::write(&invalid, b"id\n\xff\n").unwrap();
        assert!(matches!(
            read_csv_bounded(&invalid, 64, 10, 16),
            Err(CsvFileError::InvalidUtf8 { byte_offset: 3 })
        ));
        std::fs::remove_dir_all(dir).unwrap();
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
    fn explicit_types_parse_before_mutation_review() {
        let table = parse_csv("id,ratio,active,name\n7,1.5,true,Ada\n", 4, 32).unwrap();
        let changes = csv_to_typed_insert_changes(
            &table,
            &[
                CsvValueType::Signed,
                CsvValueType::Float64,
                CsvValueType::Boolean,
                CsvValueType::Text,
            ],
            32,
        )
        .unwrap();
        let MutationChange::InsertRow { values } = &changes[0] else {
            panic!("expected insert")
        };
        assert!(matches!(
            values[0].value().as_ref(),
            tablerock_core::ValueRef::Signed(7)
        ));
        assert!(matches!(
            values[2].value().as_ref(),
            tablerock_core::ValueRef::Boolean(true)
        ));
        assert!(csv_to_typed_insert_changes(&table, &[CsvValueType::Text; 4], 32).is_ok());
        assert!(csv_to_typed_insert_changes(&table, &[CsvValueType::Signed; 4], 32).is_err());
    }

    #[test]
    fn csv_insert_changes_form_valid_mutation_plan_without_sql() {
        use tablerock_core::{
            ContextId, Engine, IdParts, MutationId, MutationPlan, MutationPlanLimits,
            MutationTarget, OperationScope, ProfileId, Revision, SessionId,
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
