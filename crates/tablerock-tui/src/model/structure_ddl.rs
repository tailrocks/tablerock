//! Extract Rust-owned copyable DDL from structure inspector text.

/// Quote a PostgreSQL identifier (double-quote, escape internal quotes).
#[must_use]
pub fn quote_ident_sql(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

/// Extract bounded DDL emitted by the shared engine structure snapshot.
pub fn compose_create_table_ddl(
    schema: &str,
    table: &str,
    structure_text: &str,
) -> Result<String, String> {
    if schema.is_empty() || table.is_empty() {
        return Err("schema and table required".into());
    }
    let (_, tail) = structure_text
        .split_once("-- ddl --\n")
        .ok_or_else(|| "structure DDL unavailable".to_owned())?;
    let out = tail
        .split("\n--- quick actions ---")
        .next()
        .unwrap_or(tail)
        .trim();
    if out.is_empty() || out == "(unavailable)" {
        return Err("structure DDL unavailable".into());
    }
    if out.len() > 256 * 1024 {
        return Err("DDL exceeds copy size limit".into());
    }
    Ok(out.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_create_table_from_structure_lines() {
        let text = "-- columns --\nid integer\n-- ddl --\nCREATE TABLE x (id integer);\n--- quick actions ---\nCopyDdl";
        let ddl = compose_create_table_ddl("public", "users", text).unwrap();
        assert_eq!(ddl, "CREATE TABLE x (id integer);");
    }

    #[test]
    fn compose_fails_without_columns() {
        assert!(compose_create_table_ddl("public", "t", "-- indexes --\n(none)").is_err());
        assert!(compose_create_table_ddl("", "t", "-- columns --\nid int").is_err());
    }

    #[test]
    fn quote_ident_escapes_quotes() {
        assert_eq!(quote_ident_sql(r#"a"b"#), r#""a""b""#);
    }

    #[test]
    fn rejects_unavailable_ddl() {
        assert!(compose_create_table_ddl("public", "t", "-- ddl --\n(unavailable)").is_err());
    }
}
