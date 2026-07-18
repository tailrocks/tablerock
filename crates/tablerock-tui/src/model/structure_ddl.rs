//! Compose copyable CREATE TABLE DDL from structure inspector lines.
//!
//! Structure lines come from plan 013 ShowStructure (columns / indexes /
//! constraints sections). This is presentation-side reconstruction for
//! clipboard copy — not a second schema model.

/// Quote a PostgreSQL identifier (double-quote, escape internal quotes).
#[must_use]
pub fn quote_ident_sql(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

/// Build CREATE TABLE + trailing index statements from structure panel text.
///
/// Returns error when no column section/lines are present.
pub fn compose_create_table_ddl(
    schema: &str,
    table: &str,
    structure_text: &str,
) -> Result<String, String> {
    if schema.is_empty() || table.is_empty() {
        return Err("schema and table required".into());
    }
    let mut section = Section::None;
    let mut columns: Vec<String> = Vec::new();
    let mut constraints: Vec<String> = Vec::new();
    let mut indexes: Vec<String> = Vec::new();

    for raw in structure_text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("---") {
            continue;
        }
        if line.eq_ignore_ascii_case("-- columns --") {
            section = Section::Columns;
            continue;
        }
        if line.eq_ignore_ascii_case("-- indexes --") {
            section = Section::Indexes;
            continue;
        }
        if line.eq_ignore_ascii_case("-- constraints --") {
            section = Section::Constraints;
            continue;
        }
        if line == "(none)" {
            continue;
        }
        match section {
            Section::None => {}
            Section::Columns => columns.push(line.to_owned()),
            Section::Indexes => {
                // "PRIMARY name: CREATE INDEX ..." or similar — keep DDL after first ": ".
                if let Some((_, def)) = line.split_once(": ") {
                    let def = def.trim();
                    if def.to_ascii_uppercase().starts_with("CREATE ") {
                        indexes.push(format!("{def};"));
                    }
                } else if line.to_ascii_uppercase().starts_with("CREATE ") {
                    indexes.push(if line.ends_with(';') {
                        line.to_owned()
                    } else {
                        format!("{line};")
                    });
                }
            }
            Section::Constraints => {
                // "PRIMARY KEY name: PRIMARY KEY (id)" → CONSTRAINT "name" PRIMARY KEY (id)
                if let Some((kind_name, def)) = line.split_once(": ") {
                    let def = def.trim();
                    let name = kind_name
                        .split_whitespace()
                        .last()
                        .unwrap_or("constraint");
                    // Skip if def already is a full CREATE (indexes section owns those).
                    if def.to_ascii_uppercase().starts_with("CREATE ") {
                        continue;
                    }
                    constraints.push(format!(
                        "CONSTRAINT {} {}",
                        quote_ident_sql(name),
                        def
                    ));
                }
            }
        }
    }

    if columns.is_empty() {
        return Err("no column definitions in structure".into());
    }

    let mut body_parts = Vec::with_capacity(columns.len() + constraints.len());
    for col in &columns {
        body_parts.push(format!("  {col}"));
    }
    for c in &constraints {
        body_parts.push(format!("  {c}"));
    }

    let mut out = String::new();
    out.push_str(&format!(
        "CREATE TABLE {}.{} (\n{}\n);",
        quote_ident_sql(schema),
        quote_ident_sql(table),
        body_parts.join(",\n")
    ));
    if !indexes.is_empty() {
        out.push('\n');
        for idx in &indexes {
            out.push('\n');
            out.push_str(idx);
        }
    }
    // Bound copy payload (structure already bounded server-side).
    if out.len() > 256 * 1024 {
        return Err("DDL exceeds copy size limit".into());
    }
    Ok(out)
}

#[derive(Clone, Copy)]
enum Section {
    None,
    Columns,
    Indexes,
    Constraints,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_create_table_from_structure_lines() {
        let text = "\
-- columns --
id integer NOT NULL
name text NULL DEFAULT 'x'
-- indexes --
PRIMARY users_pkey: CREATE UNIQUE INDEX users_pkey ON public.users USING btree (id)
INDEX users_name_idx: CREATE INDEX users_name_idx ON public.users USING btree (name)
-- constraints --
PRIMARY KEY users_pkey: PRIMARY KEY (id)
FOREIGN KEY users_org_fk: FOREIGN KEY (org_id) REFERENCES public.orgs(id)
--- quick actions ---
AddCol / DropCol
";
        let ddl = compose_create_table_ddl("public", "users", text).unwrap();
        assert!(ddl.starts_with("CREATE TABLE \"public\".\"users\" ("));
        assert!(ddl.contains("id integer NOT NULL"));
        assert!(ddl.contains("name text NULL DEFAULT 'x'"));
        assert!(ddl.contains("CONSTRAINT \"users_pkey\" PRIMARY KEY (id)"));
        assert!(ddl.contains("CONSTRAINT \"users_org_fk\" FOREIGN KEY"));
        assert!(ddl.contains("CREATE UNIQUE INDEX users_pkey"));
        assert!(ddl.contains("CREATE INDEX users_name_idx"));
        assert!(!ddl.contains("quick actions"));
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
}
