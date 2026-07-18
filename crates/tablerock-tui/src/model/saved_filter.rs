//! Named filter presets for a base table (in-memory + JSON round-trip).
//!
//! Persistence stores JSON only; engine re-types values when re-browsing.

use super::grid::GridFilterChip;

/// One named filter preset for schema.table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedFilterPreset {
    pub name: String,
    pub schema: String,
    pub table: String,
    pub filters: Vec<GridFilterChip>,
    pub raw_where: Option<String>,
}

/// In-memory preset library for the workbench session.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SavedFilterLibrary {
    pub presets: Vec<SavedFilterPreset>,
}

impl SavedFilterLibrary {
    pub fn upsert(&mut self, preset: SavedFilterPreset) {
        if let Some(existing) = self.presets.iter_mut().find(|p| {
            p.name == preset.name && p.schema == preset.schema && p.table == preset.table
        }) {
            *existing = preset;
        } else {
            self.presets.push(preset);
        }
    }

    pub fn get(&self, name: &str, schema: &str, table: &str) -> Option<&SavedFilterPreset> {
        self.presets
            .iter()
            .find(|p| p.name == name && p.schema == schema && p.table == table)
    }

    /// Preset names bound to a table (for apply dialog hints).
    #[must_use]
    pub fn names_for_table(&self, schema: &str, table: &str) -> Vec<String> {
        self.presets
            .iter()
            .filter(|p| p.schema == schema && p.table == table)
            .map(|p| p.name.clone())
            .collect()
    }

    /// Minimal JSON for persistence (no cells/credentials).
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::from("[");
        for (i, p) in self.presets.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                r#"{{"name":"{}","schema":"{}","table":"{}","raw_where":{},"filters":["#,
                escape_json(&p.name),
                escape_json(&p.schema),
                escape_json(&p.table),
                p.raw_where
                    .as_deref()
                    .map(|w| format!(r#""{}""#, escape_json(w)))
                    .unwrap_or_else(|| "null".into()),
            ));
            for (j, f) in p.filters.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                out.push_str(&format!(
                    r#"{{"column":"{}","operator":"{}","value":{}}}"#,
                    escape_json(&f.column),
                    escape_json(&f.operator),
                    f.value
                        .as_deref()
                        .map(|v| format!(r#""{}""#, escape_json(v)))
                        .unwrap_or_else(|| "null".into()),
                ));
            }
            out.push_str("]}");
        }
        out.push(']');
        out
    }

    /// Parse JSON produced by `to_json` (fail closed on garbage).
    pub fn from_json(json: &str) -> Option<Self> {
        if !json.trim_start().starts_with('[') {
            return None;
        }
        // Extremely small custom parse for our own writer — not a general JSON lib.
        let mut presets = Vec::new();
        let mut rest = json;
        while let Some(idx) = rest.find(r#""name""#) {
            rest = &rest[idx..];
            let name = extract_string(rest, "name")?;
            let schema = extract_string(rest, "schema")?;
            let table = extract_string(rest, "table")?;
            let raw_where = extract_optional_string(rest, "raw_where");
            let mut filters = Vec::new();
            if let Some(fstart) = rest.find(r#""filters""#) {
                let mut frest = &rest[fstart..];
                while let Some(cidx) = frest.find(r#""column""#) {
                    // Stop at next preset object if present.
                    if let Some(next_name) = frest.find(r#""name""#) {
                        if next_name < cidx {
                            break;
                        }
                    }
                    frest = &frest[cidx..];
                    let column = extract_string(frest, "column")?;
                    let operator = extract_string(frest, "operator")?;
                    let value = extract_optional_string(frest, "value");
                    filters.push(GridFilterChip {
                        column,
                        operator,
                        value,
                    });
                    frest = frest.get(8..).unwrap_or("");
                }
            }
            presets.push(SavedFilterPreset {
                name,
                schema,
                table,
                filters,
                raw_where,
            });
            rest = rest.get(8..).unwrap_or("");
        }
        Some(Self { presets })
    }
}

/// Preset names: 1..=64 of `[A-Za-z0-9._-]` (no spaces / free SQL).
#[must_use]
pub fn is_safe_preset_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn extract_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let idx = json.find(&needle)?;
    let after = &json[idx + needle.len()..];
    let colon = after.find(':')?;
    let mut rest = after[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    rest = &rest[1..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            }
            '"' => break,
            other => out.push(other),
        }
    }
    Some(out)
}

fn extract_optional_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let idx = json.find(&needle)?;
    let after = &json[idx + needle.len()..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    if rest.starts_with("null") {
        return None;
    }
    extract_string(json, key)
}

/// Relaunch policy: Manual reconnect must not auto-connect on restore.
#[must_use]
pub fn should_auto_reconnect(preference_label: &str) -> bool {
    let lower = preference_label.to_ascii_lowercase();
    lower.contains("automatic") || lower.contains("bounded")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preset_json() {
        let mut lib = SavedFilterLibrary::default();
        lib.upsert(SavedFilterPreset {
            name: "active".into(),
            schema: "public".into(),
            table: "users".into(),
            filters: vec![GridFilterChip {
                column: "status".into(),
                operator: "eq".into(),
                value: Some("open".into()),
            }],
            raw_where: None,
        });
        let json = lib.to_json();
        let restored = SavedFilterLibrary::from_json(&json).unwrap();
        assert_eq!(restored.presets.len(), 1);
        assert_eq!(restored.presets[0].name, "active");
        assert_eq!(restored.presets[0].filters[0].value.as_deref(), Some("open"));
    }

    #[test]
    fn preset_name_charset_is_restrictive() {
        assert!(is_safe_preset_name("default"));
        assert!(is_safe_preset_name("active_only"));
        assert!(is_safe_preset_name("a.b-c_1"));
        assert!(!is_safe_preset_name(""));
        assert!(!is_safe_preset_name("bad name"));
        assert!(!is_safe_preset_name("x;drop"));
        assert!(!is_safe_preset_name(&"x".repeat(65)));
    }

    #[test]
    fn names_for_table_lists_only_matching() {
        let mut lib = SavedFilterLibrary::default();
        lib.upsert(SavedFilterPreset {
            name: "a".into(),
            schema: "public".into(),
            table: "users".into(),
            filters: Vec::new(),
            raw_where: None,
        });
        lib.upsert(SavedFilterPreset {
            name: "b".into(),
            schema: "public".into(),
            table: "orders".into(),
            filters: Vec::new(),
            raw_where: None,
        });
        assert_eq!(lib.names_for_table("public", "users"), vec!["a".to_owned()]);
    }

    #[test]
    fn manual_reconnect_never_auto() {
        assert!(!should_auto_reconnect("Manual"));
        assert!(!should_auto_reconnect("manual"));
        assert!(should_auto_reconnect("BoundedAutomatic"));
    }
}
