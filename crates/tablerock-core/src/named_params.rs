//! Named SQL parameters → positional `$n` rewrite (never string-substitute values).
//!
//! Supports `:name` placeholders outside strings, quoted identifiers, dollar-quoted
//! bodies, line comments, and block comments. PostgreSQL `::type` casts are not
//! treated as parameters.

use std::{collections::BTreeMap, error::Error, fmt};

/// Maximum distinct named parameters per statement.
pub const MAX_NAMED_PARAMS: usize = 64;

/// Maximum parameter name length in bytes.
pub const MAX_PARAM_NAME_BYTES: usize = 64;

/// Result of rewriting named placeholders to `$n`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedParamPlan {
    /// SQL text with `$1`…`$n` placeholders (same name → same index).
    pub sql: String,
    /// Parameter names in first-occurrence order (1-based `$n` aligns with index+1).
    pub names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamedParamError {
    EmptyName,
    NameTooLong { actual: usize },
    TooManyParams { actual: usize },
    InvalidName { name: String },
}

impl fmt::Display for NamedParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => f.write_str("parameter name is empty"),
            Self::NameTooLong { actual } => {
                write!(
                    f,
                    "parameter name {actual} bytes exceeds max {MAX_PARAM_NAME_BYTES}"
                )
            }
            Self::TooManyParams { actual } => {
                write!(f, "parameter count {actual} exceeds max {MAX_NAMED_PARAMS}")
            }
            Self::InvalidName { name } => write!(f, "invalid parameter name '{name}'"),
        }
    }
}

impl Error for NamedParamError {}

/// Rewrite `:name` placeholders to positional `$n` for prepare/bind.
///
/// Values are never inlined; callers must bind by `names` order.
pub fn rewrite_named_params(sql: &str) -> Result<NamedParamPlan, NamedParamError> {
    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len());
    let mut names: Vec<String> = Vec::new();
    let mut index_of: BTreeMap<String, usize> = BTreeMap::new();
    let mut i = 0usize;
    while i < bytes.len() {
        // Line comment
        if bytes[i] == b'-' && bytes.get(i + 1) == Some(&b'-') {
            let start = i;
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            out.push_str(&sql[start..i]);
            continue;
        }
        // Block comment
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
            let start = i;
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < bytes.len() {
                i += 2;
            }
            out.push_str(&sql[start..i.min(bytes.len())]);
            continue;
        }
        // Single-quoted string
        if bytes[i] == b'\'' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\'' {
                    if bytes.get(i + 1) == Some(&b'\'') {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push_str(&sql[start..i]);
            continue;
        }
        // Double-quoted identifier
        if bytes[i] == b'"' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'"' {
                    if bytes.get(i + 1) == Some(&b'"') {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push_str(&sql[start..i]);
            continue;
        }
        // Dollar-quoted string $tag$...$tag$
        if bytes[i] == b'$' {
            if let Some((tag_end, body_start)) = dollar_tag_end(bytes, i) {
                let tag = &sql[i..tag_end];
                let mut j = body_start;
                let mut closed = false;
                while j + tag.len() <= bytes.len() {
                    if &sql[j..j + tag.len()] == tag {
                        j += tag.len();
                        closed = true;
                        break;
                    }
                    j += 1;
                }
                out.push_str(&sql[i..if closed { j } else { bytes.len() }]);
                i = if closed { j } else { bytes.len() };
                continue;
            }
        }
        // Named param :name (not :: cast, not :=). After a `:` (as in `::int`)
        // the second colon must not start a parameter.
        if bytes[i] == b':'
            && (i == 0 || bytes[i - 1] != b':')
            && bytes.get(i + 1) != Some(&b':')
            && bytes.get(i + 1) != Some(&b'=')
            && bytes
                .get(i + 1)
                .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
        {
            let name_start = i + 1;
            let mut j = name_start;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            let name = &sql[name_start..j];
            if name.is_empty() {
                return Err(NamedParamError::EmptyName);
            }
            if name.len() > MAX_PARAM_NAME_BYTES {
                return Err(NamedParamError::NameTooLong { actual: name.len() });
            }
            if !is_valid_name(name) {
                return Err(NamedParamError::InvalidName {
                    name: name.to_owned(),
                });
            }
            let idx = if let Some(&existing) = index_of.get(name) {
                existing
            } else {
                if names.len() >= MAX_NAMED_PARAMS {
                    return Err(NamedParamError::TooManyParams {
                        actual: names.len() + 1,
                    });
                }
                let n = names.len() + 1;
                names.push(name.to_owned());
                index_of.insert(name.to_owned(), n);
                n
            };
            out.push('$');
            out.push_str(&idx.to_string());
            i = j;
            continue;
        }
        out.push(sql[i..].chars().next().unwrap());
        i += sql[i..].chars().next().unwrap().len_utf8();
    }
    Ok(NamedParamPlan { sql: out, names })
}

fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Return (end index exclusive of `$tag$`, body start) when `bytes[i]` is `$`.
fn dollar_tag_end(bytes: &[u8], i: usize) -> Option<(usize, usize)> {
    if bytes.get(i) != Some(&b'$') {
        return None;
    }
    let mut j = i + 1;
    while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
        j += 1;
    }
    if j < bytes.len() && bytes[j] == b'$' {
        Some((j + 1, j + 1))
    } else {
        None
    }
}

/// Parse `name=value` pairs (`;` or newline separated) into a name→value map.
///
/// Empty values are allowed (`name=`). Duplicate names: last wins.
pub fn parse_param_bindings(raw: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in raw.split(|c| c == ';' || c == '\n') {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            if !k.is_empty() {
                map.insert(k.to_owned(), v.trim().to_owned());
            }
        }
    }
    map
}

/// Resolve plan names against a binding map; missing names returned as error list.
pub fn bind_named_values(
    plan: &NamedParamPlan,
    bindings: &BTreeMap<String, String>,
) -> Result<Vec<String>, Vec<String>> {
    let mut missing = Vec::new();
    let mut values = Vec::with_capacity(plan.names.len());
    for name in &plan.names {
        match bindings.get(name) {
            Some(v) => values.push(v.clone()),
            None => missing.push(name.clone()),
        }
    }
    if missing.is_empty() {
        Ok(values)
    } else {
        Err(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_named_and_skips_casts() {
        let plan =
            rewrite_named_params("SELECT :id::int, :name, :id FROM t WHERE x = :name").unwrap();
        assert_eq!(plan.names, vec!["id".to_owned(), "name".to_owned()]);
        assert_eq!(plan.sql, "SELECT $1::int, $2, $1 FROM t WHERE x = $2");
    }

    #[test]
    fn ignores_params_inside_strings_and_comments() {
        let plan = rewrite_named_params("SELECT ':not' /* :nope */, -- :no\n :yes FROM t").unwrap();
        assert_eq!(plan.names, vec!["yes".to_owned()]);
        assert!(plan.sql.contains("$1"));
        assert!(plan.sql.contains("':not'"));
    }

    #[test]
    fn bind_values_missing_reported() {
        let plan = rewrite_named_params("SELECT :a, :b").unwrap();
        let mut m = BTreeMap::new();
        m.insert("a".into(), "1".into());
        let err = bind_named_values(&plan, &m).unwrap_err();
        assert_eq!(err, vec!["b".to_owned()]);
    }

    #[test]
    fn parse_bindings_semicolon_and_newline() {
        let m = parse_param_bindings("id=42; name=alice\n#c\nflag=true");
        assert_eq!(m.get("id").map(String::as_str), Some("42"));
        assert_eq!(m.get("name").map(String::as_str), Some("alice"));
        assert_eq!(m.get("flag").map(String::as_str), Some("true"));
    }
}
