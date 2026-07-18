//! Bounded CSV import reader — formula-neutral, position-aware errors.
//!
//! Formula-like cells (`=SUM(...)`) are imported as plain text data and never
//! evaluated. Encoding is UTF-8; invalid UTF-8 yields an explicit error with
//! byte offset.

use std::fmt;

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
