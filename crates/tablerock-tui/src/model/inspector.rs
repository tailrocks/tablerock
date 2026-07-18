//! Cell inspector projection (text / hex / structured label).

use super::grid::{CellDistinction, ProjectedCell};

/// Full-value inspector for the selected cell.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InspectorModel {
    pub open: bool,
    pub title: String,
    pub kind_label: String,
    pub text: String,
    pub hex: String,
    pub byte_len: u64,
    pub original_byte_len: Option<u64>,
    pub stale: bool,
    /// When structure is open, DDL quick actions target this relation.
    pub structure_schema: Option<String>,
    pub structure_table: Option<String>,
}

impl InspectorModel {
    #[must_use]
    pub fn from_cell(title: impl Into<String>, cell: &ProjectedCell, stale: bool) -> Self {
        let hex = format_hex_panel(cell);
        let text = match cell.distinction {
            CellDistinction::Structured => pretty_structured(&cell.text),
            CellDistinction::Temporal => format!(
                "{}\n(Today / Now · null: SetNull)",
                annotate_temporal(&cell.text)
            ),
            CellDistinction::Boolean => format!(
                "{}\n(toggle: TogBool · null: SetNull)",
                cell.display()
            ),
            CellDistinction::Binary => {
                let shown = hex_byte_count(cell).min(256);
                format!(
                    "{}\n(binary · hex panel shows first {shown} of {} bytes · CopyHex)",
                    cell.display(),
                    cell.byte_len.max(cell.text.len() as u64)
                )
            }
            _ => cell.display(),
        };
        Self {
            open: true,
            title: title.into(),
            kind_label: cell.distinction.label().into(),
            text,
            hex,
            byte_len: cell.byte_len,
            original_byte_len: cell.original_byte_len,
            stale,
            structure_schema: None,
            structure_table: None,
        }
    }

    /// True when this inspector holds a relation structure target for DDL.
    #[must_use]
    pub fn has_structure_target(&self) -> bool {
        self.open
            && self.kind_label == "structure"
            && self.structure_schema.is_some()
            && self.structure_table.is_some()
    }

    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        if !self.open {
            return Vec::new();
        }
        let mut out = vec![
            format!("inspector: {}", self.title),
            format!("kind: {}", self.kind_label),
            format!("bytes: {}", self.byte_len),
        ];
        if let Some(orig) = self.original_byte_len {
            out.push(format!("original bytes: {orig} (truncated)"));
        }
        if self.stale {
            out.push("stale: yes".into());
        }
        if self.kind_label == "explain" || looks_like_explain_plan(&self.text) {
            out.push("plan:".into());
            for line in explain_tree_lines(&self.text) {
                out.push(line);
            }
        } else if self.kind_label == "structured" {
            out.push("tree:".into());
            for line in structured_tree_lines(&self.text) {
                out.push(line);
            }
        } else if self.kind_label == "structure" || self.text.contains('\n') {
            out.push(if self.kind_label == "structure" {
                "structure:".into()
            } else {
                "text:".into()
            });
            // Cap structure dump for the panel; full text remains for CopyDdl.
            for line in self.text.lines().take(120) {
                out.push(line.to_owned());
            }
            if self.text.lines().count() > 120 {
                out.push("… (structure truncated in panel · use CopyDdl)".into());
            }
        } else {
            out.push(format!("text: {}", self.text));
        }
        if !self.hex.is_empty() {
            if self.hex.contains('\n') {
                out.push("hex:".into());
                for line in self.hex.lines().take(20) {
                    out.push(line.to_owned());
                }
                if self.hex.lines().count() > 20 {
                    out.push("…".into());
                }
            } else {
                out.push(format!("hex: {}", self.hex));
            }
        }
        out
    }

    /// Build inspector from EXPLAIN result text (multi-line plan).
    #[must_use]
    pub fn from_explain_text(title: impl Into<String>, plan_text: &str) -> Self {
        Self {
            open: true,
            title: title.into(),
            kind_label: "explain".into(),
            text: plan_text.to_owned(),
            hex: String::new(),
            byte_len: plan_text.len() as u64,
            original_byte_len: None,
            stale: false,
            structure_schema: None,
            structure_table: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::grid::{CellDistinction, ProjectedCell};

    #[test]
    fn inspector_marks_truncation_and_stale() {
        let cell = ProjectedCell {
            text: "hello".into(),
            distinction: CellDistinction::Truncated,
            byte_len: 5,
            original_byte_len: Some(50),
        };
        let insp = InspectorModel::from_cell("users.id", &cell, true);
        let lines = insp.lines().join("\n");
        assert!(lines.contains("truncated"));
        assert!(lines.contains("stale: yes"));
        assert!(lines.contains("kind: truncated"));
    }

    #[test]
    fn binary_hex_panel_is_multiline_dump() {
        let cell = ProjectedCell {
            text: "Hello, binary!\n\0\x01\x02".into(),
            distinction: CellDistinction::Binary,
            byte_len: 18,
            original_byte_len: None,
        };
        let insp = InspectorModel::from_cell("row.blob", &cell, false);
        assert!(insp.hex.contains("0000"));
        assert!(insp.hex.contains('|'));
        assert!(insp.hex.contains("48 65 6c 6c 6f")); // Hello
        let lines = insp.lines().join("\n");
        assert!(lines.contains("hex:"));
        assert!(insp.text.contains("binary"));
    }

    #[test]
    fn structured_pretty_print_and_temporal_annotation() {
        let json = ProjectedCell {
            text: r#"{"a":1,"b":true}"#.into(),
            distinction: CellDistinction::Structured,
            byte_len: 15,
            original_byte_len: None,
        };
        let insp = InspectorModel::from_cell("row.payload", &json, false);
        assert!(insp.text.contains('\n') || insp.text.contains("  "));
        assert!(insp.text.contains("\"a\""));
        let lines = insp.lines().join("\n");
        assert!(lines.contains("tree:"));
        assert!(lines.contains("\"a\""));

        let temp = ProjectedCell {
            text: "2024-01-15T12:30:00Z".into(),
            distinction: CellDistinction::Temporal,
            byte_len: 20,
            original_byte_len: None,
        };
        let t = InspectorModel::from_cell("row.ts", &temp, false);
        assert!(t.text.contains("date:"));
        assert!(t.text.contains("2024-01-15"));
    }

    #[test]
    fn pretty_structured_invalid_falls_back() {
        assert_eq!(pretty_structured("not-json"), "not-json");
        assert_eq!(structured_tree_lines("not-json"), vec!["not-json".to_owned()]);
    }

    #[test]
    fn structured_tree_caps_depth() {
        // Nested arrays deeper than MAX_STRUCTURED_TREE_DEPTH collapse.
        let deep = "[[[[[[[[[[1]]]]]]]]]]";
        let lines = structured_tree_lines(deep);
        let joined = lines.join("\n");
        assert!(
            joined.contains('…'),
            "expected collapse marker in {joined:?}"
        );
        assert!(lines.len() <= 64);
    }

    #[test]
    fn explain_tree_uses_indent_glyphs() {
        let plan = "Seq Scan on t  (cost=0.00..1.00 rows=1)\n  Filter: (id = 1)\n  ->  Index Scan on t_pkey";
        let lines = explain_tree_lines(plan);
        assert!(lines.iter().any(|l| l.contains("Seq Scan")));
        assert!(lines.iter().any(|l| l.contains("│") || l.contains("└") || l.contains("  ")));
        let insp = InspectorModel::from_explain_text("explain", plan);
        let joined = insp.lines().join("\n");
        assert!(joined.contains("plan:"));
        assert!(joined.contains("Seq Scan"));
    }
}

fn looks_like_explain_plan(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("seq scan")
        || lower.contains("index scan")
        || lower.contains("cost=")
        || lower.contains("hash join")
        || lower.contains("nested loop")
}

/// Project PostgreSQL-style EXPLAIN FORMAT TEXT into tree-ish lines.
fn explain_tree_lines(plan: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in plan.lines() {
        if raw.trim().is_empty() {
            continue;
        }
        // Count leading spaces (2-space indent convention).
        let indent = raw.chars().take_while(|c| *c == ' ').count() / 2;
        let body = raw.trim_start();
        let mut prefix = String::new();
        for i in 0..indent {
            if i + 1 == indent {
                prefix.push_str("└─ ");
            } else {
                prefix.push_str("│  ");
            }
        }
        out.push(format!("{prefix}{body}"));
    }
    if out.is_empty() {
        out.push(plan.to_owned());
    }
    out
}

/// Max nesting shown as expanded tree lines before collapsing remainder.
const MAX_STRUCTURED_TREE_DEPTH: i32 = 6;
/// Cap pretty output growth (invalid/huge payloads stay bounded).
const MAX_STRUCTURED_PRETTY_BYTES: usize = 16 * 1024;

/// Multi-line tree projection for structured cells (glyph indent + depth cap).
fn structured_tree_lines(raw: &str) -> Vec<String> {
    let pretty = pretty_structured(raw);
    if pretty == raw && !(raw.trim().starts_with('{') || raw.trim().starts_with('[')) {
        return vec![raw.to_owned()];
    }
    let mut out = Vec::new();
    for line in pretty.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.chars().take_while(|c| *c == ' ').count() / 2;
        let body = line.trim_start();
        let mut prefix = String::new();
        for i in 0..indent {
            if i + 1 == indent {
                prefix.push_str("└─ ");
            } else {
                prefix.push_str("│  ");
            }
        }
        out.push(format!("{prefix}{body}"));
        if out.len() >= 64 {
            out.push("└─ … (tree truncated)".into());
            break;
        }
    }
    if out.is_empty() {
        out.push(raw.to_owned());
    }
    out
}

/// How many payload bytes the hex panel will show (capped).
fn hex_byte_count(cell: &ProjectedCell) -> usize {
    let bytes = hex_source_bytes(cell);
    bytes.len().min(256)
}

/// Bytes to dump: prefer raw UTF-8 of cell text (presentation bytes).
fn hex_source_bytes(cell: &ProjectedCell) -> Vec<u8> {
    // Binary cells may already store space-separated hex; keep as UTF-8 view of text.
    cell.text.as_bytes().to_vec()
}

/// Multi-line hex dump: 16 bytes per line with offset, max 256 bytes.
fn format_hex_panel(cell: &ProjectedCell) -> String {
    let bytes = hex_source_bytes(cell);
    if bytes.is_empty() {
        return String::new();
    }
    let limit = bytes.len().min(256);
    let slice = &bytes[..limit];
    let mut lines = Vec::new();
    for (chunk_i, chunk) in slice.chunks(16).enumerate() {
        let off = chunk_i * 16;
        let mut hex = String::new();
        let mut ascii = String::new();
        for (j, b) in chunk.iter().enumerate() {
            if j > 0 {
                hex.push(' ');
            }
            hex.push_str(&format!("{b:02x}"));
            let c = *b as char;
            if c.is_ascii_graphic() || c == ' ' {
                ascii.push(c);
            } else {
                ascii.push('.');
            }
        }
        // Pad hex to fixed width for alignment when last line short.
        while hex.len() < 16 * 3 - 1 {
            hex.push(' ');
        }
        lines.push(format!("{off:04x}  {hex}  |{ascii}|"));
    }
    if bytes.len() > limit {
        lines.push(format!("… ({} more bytes not shown)", bytes.len() - limit));
    }
    lines.join("\n")
}

/// Indent JSON-like structured text for inspector readability (best-effort).
///
/// Depth beyond [`MAX_STRUCTURED_TREE_DEPTH`] is collapsed to `…` so nested
/// containers remain navigable without dumping unbounded trees.
fn pretty_structured(raw: &str) -> String {
    let trimmed = raw.trim();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return raw.to_owned();
    }
    let mut out = String::with_capacity(raw.len().min(MAX_STRUCTURED_PRETTY_BYTES) + 32);
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut escape = false;
    let mut collapse_until: Option<i32> = None;
    let bytes = trimmed.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if out.len() >= MAX_STRUCTURED_PRETTY_BYTES {
            out.push_str("\n…");
            break;
        }
        let b = bytes[i];
        if in_str {
            out.push(b as char);
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if let Some(floor) = collapse_until {
            // Skip nested content until we close back to the collapse floor.
            match b {
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    depth = depth.saturating_sub(1);
                    if depth <= floor {
                        collapse_until = None;
                        out.push(b as char);
                    }
                }
                b'"' => {
                    // Skip string fully while collapsed.
                    i += 1;
                    let mut esc = false;
                    while i < bytes.len() {
                        let c = bytes[i];
                        if esc {
                            esc = false;
                        } else if c == b'\\' {
                            esc = true;
                        } else if c == b'"' {
                            break;
                        }
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
            continue;
        }
        match b {
            b'"' => {
                in_str = true;
                out.push('"');
            }
            b'{' | b'[' => {
                if depth >= MAX_STRUCTURED_TREE_DEPTH {
                    out.push(b as char);
                    out.push('…');
                    collapse_until = Some(depth);
                    depth += 1;
                } else {
                    out.push(b as char);
                    depth += 1;
                    out.push('\n');
                    for _ in 0..depth {
                        out.push_str("  ");
                    }
                }
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                out.push('\n');
                for _ in 0..depth {
                    out.push_str("  ");
                }
                out.push(b as char);
            }
            b',' => {
                out.push(',');
                out.push('\n');
                for _ in 0..depth {
                    out.push_str("  ");
                }
                // skip following space
                if bytes.get(i + 1) == Some(&b' ') {
                    i += 1;
                }
            }
            b':' => {
                out.push(':');
                out.push(' ');
                if bytes.get(i + 1) == Some(&b' ') {
                    i += 1;
                }
            }
            b' ' | b'\n' | b'\t' | b'\r' => {}
            _ => out.push(b as char),
        }
        i += 1;
    }
    out
}

/// Annotate ISO-like temporal values with component lines.
fn annotate_temporal(raw: &str) -> String {
    let t = raw.trim();
    let mut lines = vec![t.to_owned()];
    let (date, rest) = if let Some((d, r)) = t.split_once('T') {
        (Some(d), Some(r))
    } else if let Some((d, r)) = t.split_once(' ') {
        (Some(d), Some(r))
    } else if t.len() >= 10 && t.as_bytes().get(4) == Some(&b'-') {
        (Some(&t[..10]), if t.len() > 10 { Some(&t[10..]) } else { None })
    } else {
        (None, Some(t))
    };
    if let Some(d) = date {
        let parts: Vec<_> = d.split('-').collect();
        if parts.len() == 3 {
            lines.push(format!(
                "date: {d} (y={} m={} d={})",
                parts[0], parts[1], parts[2]
            ));
        } else {
            lines.push(format!("date: {d}"));
        }
    }
    if let Some(r) = rest {
        let r = r.trim_start_matches(|c: char| c == 'T' || c == ' ');
        if r.is_empty() {
            return lines.join("\n");
        }
        let tz = if r.ends_with('Z') {
            Some("UTC")
        } else if let Some(pos) = r.char_indices().rev().find(|(_, c)| *c == '+' || *c == '-') {
            // timezone offset starts at last + or - after time body
            if pos.0 >= 8 {
                Some(&r[pos.0..])
            } else {
                None
            }
        } else {
            None
        };
        let body = if r.ends_with('Z') {
            &r[..r.len() - 1]
        } else if let Some(tz_s) = tz {
            r.strip_suffix(tz_s).unwrap_or(r)
        } else {
            r
        };
        let (clock, frac) = body
            .split_once('.')
            .map(|(a, b)| (a, Some(b)))
            .unwrap_or((body, None));
        if !clock.is_empty() {
            lines.push(format!("time: {clock}"));
        }
        if let Some(f) = frac {
            if f.chars().all(|c| c.is_ascii_digit()) {
                lines.push(format!("fraction: {f}"));
            }
        }
        if let Some(tz_s) = tz {
            lines.push(format!("tz: {tz_s}"));
        }
    }
    lines.join("\n")
}
