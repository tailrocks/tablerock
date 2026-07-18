//! Identifier quoting for engine-built SQL fragments.
//!
//! Never concatenate user text into SQL. Use [`quote_ident`] for PostgreSQL
//! identifiers (schema/table/column names already validated as names).

/// Quote a PostgreSQL identifier with double quotes, doubling internal quotes.
///
/// Rejects empty names and NULs. Does not validate Unicode beyond UTF-8.
pub fn quote_ident(name: &str) -> Result<String, QuoteIdentError> {
    if name.is_empty() {
        return Err(QuoteIdentError::Empty);
    }
    if name.bytes().any(|b| b == 0) {
        return Err(QuoteIdentError::Nul);
    }
    let mut out = String::with_capacity(name.len() + 2);
    out.push('"');
    for ch in name.chars() {
        if ch == '"' {
            out.push('"');
            out.push('"');
        } else {
            out.push(ch);
        }
    }
    out.push('"');
    Ok(out)
}

/// Qualify `schema.table` with both identifiers quoted.
pub fn qualify_table(schema: &str, table: &str) -> Result<String, QuoteIdentError> {
    Ok(format!("{}.{}", quote_ident(schema)?, quote_ident(table)?))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteIdentError {
    Empty,
    Nul,
}

impl std::fmt::Display for QuoteIdentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Empty => "identifier is empty",
            Self::Nul => "identifier contains NUL",
        })
    }
}

impl std::error::Error for QuoteIdentError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_and_escapes() {
        assert_eq!(quote_ident("users").unwrap(), "\"users\"");
        assert_eq!(quote_ident("weird\"name").unwrap(), "\"weird\"\"name\"");
        assert_eq!(
            qualify_table("public", "users").unwrap(),
            "\"public\".\"users\""
        );
    }

    #[test]
    fn rejects_empty_and_nul() {
        assert_eq!(quote_ident(""), Err(QuoteIdentError::Empty));
        assert_eq!(quote_ident("a\0b"), Err(QuoteIdentError::Nul));
    }
}
