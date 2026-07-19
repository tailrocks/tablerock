use std::fmt;

use crate::{CellRef, Truncation, ValueKind};

const MAX_COPY_BYTES: usize = 16 * 1024 * 1024;
const MAX_COPY_ROWS: usize = 10_000;
const MAX_COPY_COLUMNS: usize = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyFormat {
    Csv,
    Tsv,
    Json,
    Markdown,
    SqlInsert,
    SqlUpdate,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CopyCell {
    Null,
    Boolean(bool),
    Number(String),
    Text(String),
    Binary(Vec<u8>),
    Unknown(Vec<u8>),
    Invalid(Vec<u8>),
    Truncated {
        display: String,
        original_bytes: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CopyTable {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<CopyCell>>,
    pub base_schema: Option<String>,
    pub base_table: Option<String>,
    pub identity_columns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyProjectionError {
    Empty,
    ShapeMismatch,
    BoundsExceeded,
    MissingTableIdentity,
    MissingStableIdentity,
}

impl fmt::Display for CopyProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "nothing to copy",
            Self::ShapeMismatch => "copy table row shape does not match columns",
            Self::BoundsExceeded => "copy payload exceeds row, column, or 16 MiB output bounds",
            Self::MissingTableIdentity => "SQL copy requires base-table identity",
            Self::MissingStableIdentity => "SQL UPDATE copy requires stable identity columns",
        })
    }
}

impl std::error::Error for CopyProjectionError {}

pub fn format_copy_table(
    table: &CopyTable,
    format: CopyFormat,
) -> Result<String, CopyProjectionError> {
    validate(table)?;
    let output = match format {
        CopyFormat::Csv => format_csv(table),
        CopyFormat::Tsv => format_tsv(table),
        CopyFormat::Json => format_json(table),
        CopyFormat::Markdown => format_markdown(table),
        CopyFormat::SqlInsert => format_sql_insert(table)?,
        CopyFormat::SqlUpdate => format_sql_update(table)?,
    };
    if output.len() > MAX_COPY_BYTES {
        return Err(CopyProjectionError::BoundsExceeded);
    }
    Ok(output)
}

#[must_use]
pub fn copy_cell_from_page(cell: CellRef<'_>) -> CopyCell {
    let base = match cell.kind() {
        ValueKind::Null => CopyCell::Null,
        ValueKind::Boolean => {
            CopyCell::Boolean(cell.bytes().first().is_some_and(|byte| *byte != 0))
        }
        ValueKind::Signed => CopyCell::Number(decode_signed(cell.bytes())),
        ValueKind::Unsigned => CopyCell::Number(decode_unsigned(cell.bytes()).to_string()),
        ValueKind::Float64 => {
            CopyCell::Number(f64::from_bits(decode_unsigned(cell.bytes())).to_string())
        }
        ValueKind::Decimal => CopyCell::Number(text_or_hex(cell.bytes())),
        ValueKind::Temporal | ValueKind::Text | ValueKind::Structured => {
            CopyCell::Text(text_or_hex(cell.bytes()))
        }
        ValueKind::Binary => CopyCell::Binary(cell.bytes().to_vec()),
        ValueKind::Unknown => CopyCell::Unknown(cell.bytes().to_vec()),
        ValueKind::Invalid => CopyCell::Invalid(cell.bytes().to_vec()),
    };
    match cell.truncation() {
        Truncation::Complete => base,
        Truncation::Truncated { original_byte_len } => CopyCell::Truncated {
            display: display(&base),
            original_bytes: original_byte_len,
        },
    }
}

fn decode_unsigned(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(0_u64, |value, byte| (value << 8) | u64::from(*byte))
}

fn decode_signed(bytes: &[u8]) -> String {
    if bytes.is_empty() || bytes.len() > 8 {
        return format!("<signed:{}>", hex(bytes));
    }
    let unsigned = decode_unsigned(bytes);
    let shift = (8 - bytes.len()) * 8;
    ((unsigned << shift) as i64 >> shift).to_string()
}

fn text_or_hex(bytes: &[u8]) -> String {
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .unwrap_or_else(|_| format!("\\x{}", hex(bytes)))
}

fn validate(table: &CopyTable) -> Result<(), CopyProjectionError> {
    if table.columns.is_empty() || table.rows.is_empty() {
        return Err(CopyProjectionError::Empty);
    }
    if table.columns.len() > MAX_COPY_COLUMNS || table.rows.len() > MAX_COPY_ROWS {
        return Err(CopyProjectionError::BoundsExceeded);
    }
    if table
        .rows
        .iter()
        .any(|row| row.len() != table.columns.len())
    {
        return Err(CopyProjectionError::ShapeMismatch);
    }
    Ok(())
}

fn display(cell: &CopyCell) -> String {
    match cell {
        CopyCell::Null => "NULL".into(),
        CopyCell::Boolean(value) => value.to_string(),
        CopyCell::Number(value) | CopyCell::Text(value) => value.clone(),
        CopyCell::Binary(bytes) => format!("\\x{}", hex(bytes)),
        CopyCell::Unknown(bytes) => format!("<unknown:{}>", hex(bytes)),
        CopyCell::Invalid(bytes) => format!("<invalid:{}>", hex(bytes)),
        CopyCell::Truncated {
            display,
            original_bytes,
        } => match original_bytes {
            Some(bytes) => format!("{display}… [truncated from {bytes} bytes]"),
            None => format!("{display}… [truncated]"),
        },
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn format_csv(table: &CopyTable) -> String {
    let mut out = table
        .columns
        .iter()
        .map(|value| csv_escape(value))
        .collect::<Vec<_>>()
        .join(",");
    out.push('\n');
    for row in &table.rows {
        out.push_str(
            &row.iter()
                .map(|cell| csv_escape(&display(cell)))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
    }
    out
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn format_tsv(table: &CopyTable) -> String {
    let mut out = table.columns.join("\t");
    out.push('\n');
    for row in &table.rows {
        out.push_str(
            &row.iter()
                .map(|cell| display(cell).replace(['\t', '\n', '\r'], " "))
                .collect::<Vec<_>>()
                .join("\t"),
        );
        out.push('\n');
    }
    out
}

fn format_json(table: &CopyTable) -> String {
    let mut out = String::from("[\n");
    for (row_index, row) in table.rows.iter().enumerate() {
        out.push_str("  {");
        for (column_index, (column, cell)) in table.columns.iter().zip(row).enumerate() {
            if column_index > 0 {
                out.push_str(", ");
            }
            out.push('"');
            out.push_str(&json_escape(column));
            out.push_str("\":");
            match cell {
                CopyCell::Null => out.push_str("null"),
                CopyCell::Boolean(value) => out.push_str(if *value { "true" } else { "false" }),
                CopyCell::Number(value) if is_number_token(value) => out.push_str(value),
                _ => {
                    out.push('"');
                    out.push_str(&json_escape(&display(cell)));
                    out.push('"');
                }
            }
        }
        out.push('}');
        if row_index + 1 < table.rows.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("]\n");
    out
}

fn json_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            value if value.is_control() => format!("\\u{:04x}", value as u32).chars().collect(),
            value => vec![value],
        })
        .collect()
}

fn format_markdown(table: &CopyTable) -> String {
    let mut out = String::from("|");
    for column in &table.columns {
        out.push_str(&format!(" {} |", column.replace('|', "\\|")));
    }
    out.push_str("\n|");
    for _ in &table.columns {
        out.push_str(" --- |");
    }
    out.push('\n');
    for row in &table.rows {
        out.push('|');
        for cell in row {
            out.push_str(&format!(" {} |", display(cell).replace('|', "\\|")));
        }
        out.push('\n');
    }
    out
}

fn table_identity(table: &CopyTable) -> Result<(&str, &str), CopyProjectionError> {
    match (&table.base_schema, &table.base_table) {
        (Some(schema), Some(name)) if !schema.is_empty() && !name.is_empty() => Ok((schema, name)),
        _ => Err(CopyProjectionError::MissingTableIdentity),
    }
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn sql_literal(cell: &CopyCell) -> String {
    match cell {
        CopyCell::Null => "NULL".into(),
        CopyCell::Boolean(value) => {
            if *value {
                "TRUE".into()
            } else {
                "FALSE".into()
            }
        }
        CopyCell::Number(value) if is_number_token(value) => value.clone(),
        CopyCell::Binary(bytes) => format!("'\\x{}'", hex(bytes)),
        _ => format!("'{}'", display(cell).replace('\'', "''")),
    }
}

fn is_number_token(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let mut index = usize::from(bytes[0] == b'-');
    if index >= bytes.len() {
        return false;
    }
    if bytes[index] == b'0' {
        index += 1;
    } else if bytes[index].is_ascii_digit() && bytes[index] != b'0' {
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
    } else {
        return false;
    }
    if index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index == start {
            return false;
        }
    }
    if index < bytes.len() && matches!(bytes[index], b'e' | b'E') {
        index += 1;
        if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
            index += 1;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index == start {
            return false;
        }
    }
    index == bytes.len()
}

fn format_sql_insert(table: &CopyTable) -> Result<String, CopyProjectionError> {
    let (schema, name) = table_identity(table)?;
    let columns = table
        .columns
        .iter()
        .map(|column| quote_identifier(column))
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = String::new();
    for row in &table.rows {
        let values = row.iter().map(sql_literal).collect::<Vec<_>>().join(", ");
        out.push_str(&format!(
            "INSERT INTO {}.{} ({columns}) VALUES ({values});\n",
            quote_identifier(schema),
            quote_identifier(name)
        ));
    }
    Ok(out)
}

fn format_sql_update(table: &CopyTable) -> Result<String, CopyProjectionError> {
    let (schema, name) = table_identity(table)?;
    if table.identity_columns.is_empty() {
        return Err(CopyProjectionError::MissingStableIdentity);
    }
    let mut out = String::new();
    for row in &table.rows {
        let assignments = table
            .columns
            .iter()
            .zip(row)
            .filter(|(column, _)| !table.identity_columns.contains(column))
            .map(|(column, cell)| format!("{} = {}", quote_identifier(column), sql_literal(cell)))
            .collect::<Vec<_>>();
        let predicates = table
            .identity_columns
            .iter()
            .map(|identity| {
                let index = table
                    .columns
                    .iter()
                    .position(|column| column == identity)
                    .ok_or(CopyProjectionError::MissingStableIdentity)?;
                Ok(format!(
                    "{} = {}",
                    quote_identifier(identity),
                    sql_literal(&row[index])
                ))
            })
            .collect::<Result<Vec<_>, CopyProjectionError>>()?;
        if assignments.is_empty() {
            return Err(CopyProjectionError::MissingStableIdentity);
        }
        out.push_str(&format!(
            "UPDATE {}.{} SET {} WHERE {};\n",
            quote_identifier(schema),
            quote_identifier(name),
            assignments.join(", "),
            predicates.join(" AND ")
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> CopyTable {
        CopyTable {
            columns: vec!["id".into(), "name".into(), "payload".into()],
            rows: vec![vec![
                CopyCell::Number("1".into()),
                CopyCell::Text("a,b".into()),
                CopyCell::Null,
            ]],
            base_schema: Some("public".into()),
            base_table: Some("users".into()),
            identity_columns: vec!["id".into()],
        }
    }

    #[test]
    fn formats_all_six_with_typed_values() {
        let table = table();
        assert!(
            format_copy_table(&table, CopyFormat::Csv)
                .unwrap()
                .contains("\"a,b\"")
        );
        assert!(
            format_copy_table(&table, CopyFormat::Tsv)
                .unwrap()
                .contains("id\tname")
        );
        assert!(
            format_copy_table(&table, CopyFormat::Json)
                .unwrap()
                .contains("\"id\":1")
        );
        assert!(
            format_copy_table(&table, CopyFormat::Markdown)
                .unwrap()
                .contains("| id |")
        );
        assert!(
            format_copy_table(&table, CopyFormat::SqlInsert)
                .unwrap()
                .contains("INSERT INTO")
        );
        assert!(
            format_copy_table(&table, CopyFormat::SqlUpdate)
                .unwrap()
                .contains("WHERE \"id\" = 1")
        );
    }

    #[test]
    fn update_requires_proven_identity_and_bounds() {
        let mut table = table();
        table.identity_columns.clear();
        assert_eq!(
            format_copy_table(&table, CopyFormat::SqlUpdate),
            Err(CopyProjectionError::MissingStableIdentity)
        );
        table.rows = vec![vec![CopyCell::Null]; MAX_COPY_COLUMNS + 1];
        assert_eq!(
            format_copy_table(&table, CopyFormat::Csv),
            Err(CopyProjectionError::ShapeMismatch)
        );
    }

    #[test]
    fn binary_unknown_invalid_and_truncation_are_explicit() {
        let mut table = table();
        table.columns = vec!["b".into(), "u".into(), "i".into(), "t".into()];
        table.rows = vec![vec![
            CopyCell::Binary(vec![0, 255]),
            CopyCell::Unknown(vec![1]),
            CopyCell::Invalid(vec![2]),
            CopyCell::Truncated {
                display: "abc".into(),
                original_bytes: Some(9),
            },
        ]];
        let csv = format_copy_table(&table, CopyFormat::Csv).unwrap();
        assert!(csv.contains("\\x00ff"));
        assert!(csv.contains("<unknown:01>"));
        assert!(csv.contains("truncated from 9 bytes"));
    }

    #[test]
    fn hostile_or_non_json_number_is_quoted() {
        let mut table = table();
        table.columns = vec!["n".into()];
        for value in ["NaN", "1; DROP TABLE users", "01"] {
            table.rows = vec![vec![CopyCell::Number(value.into())]];
            let json = format_copy_table(&table, CopyFormat::Json).unwrap();
            assert!(json.contains(&format!("\"{value}\"")), "{json}");
            let insert = format_copy_table(&table, CopyFormat::SqlInsert).unwrap();
            assert!(insert.contains(&format!("'{value}'")), "{insert}");
        }
    }
}
