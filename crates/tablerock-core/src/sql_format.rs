//! Dialect-aware lightweight SQL formatting (TableRock-owned golden rules).
//!
//! Preserves string/identifier/dollar-quote/comment text byte-for-byte.
//! Outside those regions: collapses runs of horizontal whitespace to a single
//! space, uppercases a fixed keyword set, and normalizes newlines around
//! major clauses.

use crate::SqlDialect;

/// Keywords uppercased when they appear as whole words outside literals.
const KEYWORDS: &[&str] = &[
    "select",
    "from",
    "where",
    "and",
    "or",
    "not",
    "in",
    "is",
    "null",
    "as",
    "join",
    "left",
    "right",
    "inner",
    "outer",
    "on",
    "group",
    "by",
    "order",
    "limit",
    "offset",
    "insert",
    "into",
    "values",
    "update",
    "set",
    "delete",
    "create",
    "table",
    "index",
    "view",
    "drop",
    "alter",
    "with",
    "union",
    "all",
    "distinct",
    "having",
    "case",
    "when",
    "then",
    "else",
    "end",
    "exists",
    "between",
    "like",
    "ilike",
    "returning",
    "explain",
    "analyze",
];

/// Format SQL text. `dialect` reserved for future CH/Redis differences.
#[must_use]
pub fn format_sql(source: &str, _dialect: SqlDialect) -> String {
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut i = 0usize;
    let mut pending_space = false;
    while i < bytes.len() {
        // Preserve line comments
        if bytes[i] == b'-' && bytes.get(i + 1) == Some(&b'-') {
            flush_space(&mut out, &mut pending_space);
            let start = i;
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            out.push_str(&source[start..i]);
            continue;
        }
        // Preserve block comments
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
            flush_space(&mut out, &mut pending_space);
            let start = i;
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < bytes.len() {
                i += 2;
            }
            out.push_str(&source[start..i.min(bytes.len())]);
            continue;
        }
        // Strings
        if bytes[i] == b'\'' || bytes[i] == b'"' {
            flush_space(&mut out, &mut pending_space);
            let quote = bytes[i];
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == quote {
                    if bytes.get(i + 1) == Some(&quote) {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push_str(&source[start..i]);
            continue;
        }
        // Dollar quote
        if bytes[i] == b'$' {
            if let Some(tag_end) = dollar_tag(bytes, i) {
                flush_space(&mut out, &mut pending_space);
                let tag = &source[i..tag_end];
                let mut j = tag_end;
                while j + tag.len() <= bytes.len() {
                    if &source[j..j + tag.len()] == tag {
                        j += tag.len();
                        break;
                    }
                    j += 1;
                }
                out.push_str(&source[i..j.min(bytes.len())]);
                i = j.min(bytes.len());
                continue;
            }
        }
        // Newline → single newline, no pending space
        if bytes[i] == b'\n' || bytes[i] == b'\r' {
            pending_space = false;
            if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
                i += 2;
            } else {
                i += 1;
            }
            if !out.ends_with('\n') {
                out.push('\n');
            }
            continue;
        }
        // Horizontal whitespace → one space
        if bytes[i] == b' ' || bytes[i] == b'\t' {
            pending_space = true;
            i += 1;
            continue;
        }
        // Word: maybe keyword
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            flush_space(&mut out, &mut pending_space);
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &source[start..i];
            if is_keyword(word) {
                out.push_str(&word.to_ascii_uppercase());
            } else {
                out.push_str(word);
            }
            continue;
        }
        flush_space(&mut out, &mut pending_space);
        let ch = source[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out.trim().to_owned()
}

fn flush_space(out: &mut String, pending: &mut bool) {
    if *pending {
        if !out.is_empty() && !out.ends_with('\n') && !out.ends_with(' ') {
            out.push(' ');
        }
        *pending = false;
    }
}

fn is_keyword(word: &str) -> bool {
    let lower = word.to_ascii_lowercase();
    KEYWORDS.contains(&lower.as_str())
}

fn dollar_tag(bytes: &[u8], i: usize) -> Option<usize> {
    if bytes.get(i) != Some(&b'$') {
        return None;
    }
    let mut j = i + 1;
    while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
        j += 1;
    }
    if j < bytes.len() && bytes[j] == b'$' {
        Some(j + 1)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uppercases_keywords_and_collapses_space() {
        let out = format_sql("select   *  from   t  where  id=1", SqlDialect::PostgreSql);
        assert_eq!(out, "SELECT * FROM t WHERE id=1");
    }

    #[test]
    fn preserves_string_contents() {
        let out = format_sql("select  'from where'  from t", SqlDialect::PostgreSql);
        assert!(out.contains("'from where'"));
        assert!(out.starts_with("SELECT"));
    }

    #[test]
    fn preserves_line_comments() {
        let out = format_sql("select 1 -- keep me\nfrom t", SqlDialect::PostgreSql);
        assert!(out.contains("-- keep me"));
        assert!(out.contains("FROM t"));
    }
}
