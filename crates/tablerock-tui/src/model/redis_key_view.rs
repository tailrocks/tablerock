//! Type-specific Redis key view projections (presentation-local).
//!
//! Engine loads bounded pages; this module formats string/hash/list/set/zset/
//! stream cells for the shared grid/inspector without claiming totals.

use tablerock_core::RedisKeyKind;

/// Header facts for a key tab.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RedisKeyHeader {
    pub key_display: String,
    pub kind: Option<RedisKeyKind>,
    pub ttl_label: String,
    pub stale: bool,
    pub last_refresh_label: String,
}

impl RedisKeyHeader {
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let kind = self
            .kind
            .map(redis_kind_label)
            .unwrap_or("unknown");
        vec![
            format!("key: {}", self.key_display),
            format!("type: {kind}"),
            format!("ttl: {}", self.ttl_label),
            format!("refresh: {}", self.last_refresh_label),
            if self.stale {
                "stale: yes".into()
            } else {
                "stale: no".into()
            },
        ]
    }
}

#[must_use]
pub const fn redis_kind_label(kind: RedisKeyKind) -> &'static str {
    match kind {
        RedisKeyKind::Unknown => "unknown",
        RedisKeyKind::String => "string",
        RedisKeyKind::Hash => "hash",
        RedisKeyKind::List => "list",
        RedisKeyKind::Set => "set",
        RedisKeyKind::SortedSet => "zset",
        RedisKeyKind::Stream => "stream",
    }
}

/// String value projections (text / escaped / hex / JSON attempt).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringProjections {
    pub text: String,
    pub escaped: String,
    pub hex: String,
    pub json_label: String,
    pub truncated: bool,
}

impl StringProjections {
    #[must_use]
    pub fn from_bytes(raw: &[u8], truncated: bool) -> Self {
        let text = String::from_utf8_lossy(raw).into_owned();
        let escaped = escape_display(&text);
        let hex = raw
            .iter()
            .take(64)
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        let json_label = if text.trim_start().starts_with('{') || text.trim_start().starts_with('[')
        {
            if serde_json_looks_ok(&text) {
                "json: valid-ish".into()
            } else {
                "json: invalid".into()
            }
        } else {
            "json: n/a".into()
        };
        Self {
            text,
            escaped,
            hex,
            json_label,
            truncated,
        }
    }

    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let mut out = vec![
            format!("text: {}", self.text),
            format!("escaped: {}", self.escaped),
            format!("hex: {}", self.hex),
            self.json_label.clone(),
        ];
        if self.truncated {
            out.push("truncated: yes".into());
        }
        out
    }
}

fn escape_display(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn serde_json_looks_ok(s: &str) -> bool {
    // Lightweight structural check without pulling serde_json into tui if avoidable.
    // Accept balanced braces/brackets as "valid-ish"; full parse is engine-side.
    let t = s.trim();
    if t.is_empty() {
        return false;
    }
    let open = t.chars().next().unwrap();
    let close = t.chars().next_back().unwrap();
    matches!((open, close), ('{', '}') | ('[', ']'))
}

/// Generic two-column projection for hash fields, list indices, set members, zset scores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairRow {
    pub left: String,
    pub right: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RedisKeyViewModel {
    pub header: RedisKeyHeader,
    pub string: Option<StringProjections>,
    pub pairs: Vec<PairRow>,
    pub stream_lines: Vec<String>,
    pub truncated: bool,
}

impl RedisKeyViewModel {
    #[must_use]
    pub fn for_string(header: RedisKeyHeader, raw: &[u8], truncated: bool) -> Self {
        Self {
            header,
            string: Some(StringProjections::from_bytes(raw, truncated)),
            pairs: Vec::new(),
            stream_lines: Vec::new(),
            truncated,
        }
    }

    #[must_use]
    pub fn for_pairs(header: RedisKeyHeader, pairs: Vec<PairRow>, truncated: bool) -> Self {
        Self {
            header,
            string: None,
            pairs,
            stream_lines: Vec::new(),
            truncated,
        }
    }

    #[must_use]
    pub fn for_stream(header: RedisKeyHeader, lines: Vec<String>, truncated: bool) -> Self {
        Self {
            header,
            string: None,
            pairs: Vec::new(),
            stream_lines: lines,
            truncated,
        }
    }

    #[must_use]
    pub fn display_lines(&self) -> Vec<String> {
        let mut out = self.header.lines();
        if let Some(s) = &self.string {
            out.extend(s.lines());
        }
        for (i, row) in self.pairs.iter().enumerate() {
            let trunc = if row.truncated { " …" } else { "" };
            out.push(format!("{i}: {} → {}{trunc}", row.left, row.right));
        }
        for line in &self.stream_lines {
            out.push(line.clone());
        }
        if self.truncated {
            out.push("page truncated: yes".into());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_projections_mark_json_and_escape() {
        let p = StringProjections::from_bytes(b"{\"a\":1}\n", false);
        assert!(p.text.contains("a"));
        assert!(p.escaped.contains("\\n"));
        assert!(p.json_label.contains("json"));
        assert!(!p.hex.is_empty());
    }

    #[test]
    fn six_kind_labels_exist() {
        for kind in [
            RedisKeyKind::String,
            RedisKeyKind::Hash,
            RedisKeyKind::List,
            RedisKeyKind::Set,
            RedisKeyKind::SortedSet,
            RedisKeyKind::Stream,
        ] {
            assert!(!redis_kind_label(kind).is_empty());
        }
    }

    #[test]
    fn pair_and_stream_views_render() {
        let header = RedisKeyHeader {
            key_display: "k".into(),
            kind: Some(RedisKeyKind::Hash),
            ttl_label: "persistent".into(),
            stale: false,
            last_refresh_label: "now".into(),
        };
        let view = RedisKeyViewModel::for_pairs(
            header.clone(),
            vec![PairRow {
                left: "f".into(),
                right: "v".into(),
                truncated: false,
            }],
            false,
        );
        let lines = view.display_lines();
        assert!(lines.iter().any(|l| l.contains("f → v")));

        let stream = RedisKeyViewModel::for_stream(
            RedisKeyHeader {
                kind: Some(RedisKeyKind::Stream),
                ..header
            },
            vec!["1-0 field=a".into()],
            true,
        );
        assert!(stream.display_lines().iter().any(|l| l.contains("1-0")));
        assert!(stream.display_lines().iter().any(|l| l.contains("truncated")));
    }
}
