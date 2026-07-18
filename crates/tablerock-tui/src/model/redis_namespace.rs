//! Redis key namespace projection (UI-side `:` grouping).
//!
//! Binary/undecodable keys are never forced into a tree path — they land in a
//! flat group. SCAN is the only browse path; this module only projects
//! already-fetched key display strings.

/// One projected key for the sidebar/list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedRedisKey {
    /// Full key as display text (or hex for binary).
    pub full: String,
    /// Namespace segments before the leaf (empty for flat/binary).
    pub path: Vec<String>,
    /// Leaf name (last segment or full key).
    pub leaf: String,
    /// True when key was not valid UTF-8 / forced flat.
    pub binary_flat: bool,
}

/// Project raw key bytes into a namespace path using `:` separators.
pub fn project_key(raw: &[u8]) -> ProjectedRedisKey {
    match std::str::from_utf8(raw) {
        Ok(text) if !text.is_empty() => project_utf8_key(text),
        Ok(_) => ProjectedRedisKey {
            full: String::new(),
            path: Vec::new(),
            leaf: String::new(),
            binary_flat: false,
        },
        Err(_) => {
            let full = hex_display(raw);
            ProjectedRedisKey {
                full: full.clone(),
                path: Vec::new(),
                leaf: full,
                binary_flat: true,
            }
        }
    }
}

fn project_utf8_key(text: &str) -> ProjectedRedisKey {
    let parts: Vec<&str> = text.split(':').filter(|p| !p.is_empty()).collect();
    if parts.len() <= 1 {
        return ProjectedRedisKey {
            full: text.to_owned(),
            path: Vec::new(),
            leaf: text.to_owned(),
            binary_flat: false,
        };
    }
    let leaf = parts.last().unwrap().to_owned();
    let path = parts[..parts.len() - 1]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    ProjectedRedisKey {
        full: text.to_owned(),
        path,
        leaf: leaf.to_owned(),
        binary_flat: false,
    }
}

fn hex_display(raw: &[u8]) -> String {
    let take = raw.len().min(32);
    let mut out = String::from("0x");
    for b in &raw[..take] {
        out.push_str(&format!("{b:02x}"));
    }
    if raw.len() > take {
        out.push('…');
    }
    out
}

/// Group keys by first path segment (or flat bucket for binary/root).
pub fn group_by_namespace(keys: &[ProjectedRedisKey]) -> Vec<(String, Vec<usize>)> {
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, key) in keys.iter().enumerate() {
        let label = if key.binary_flat {
            "(binary)".to_owned()
        } else if key.path.is_empty() {
            "(root)".to_owned()
        } else {
            key.path[0].clone()
        };
        if let Some(entry) = groups.iter_mut().find(|(n, _)| n == &label) {
            entry.1.push(i);
        } else {
            groups.push((label, vec![i]));
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_and_root_and_binary() {
        let a = project_key(b"app:users:42");
        assert_eq!(a.path, vec!["app".to_owned(), "users".to_owned()]);
        assert_eq!(a.leaf, "42");
        assert!(!a.binary_flat);

        let root = project_key(b"simple");
        assert!(root.path.is_empty());
        assert_eq!(root.leaf, "simple");

        let bin = project_key(&[0xff, 0x00, 0x01]);
        assert!(bin.binary_flat);
        assert!(bin.path.is_empty());
        assert!(bin.full.starts_with("0x"));

        let keys = vec![a, root, bin];
        let groups = group_by_namespace(&keys);
        let labels: Vec<_> = groups.iter().map(|(n, _)| n.as_str()).collect();
        assert!(labels.contains(&"app"));
        assert!(labels.contains(&"(root)"));
        assert!(labels.contains(&"(binary)"));
    }

    #[test]
    fn deep_nesting_preserved() {
        let k = project_key(b"a:b:c:d:e");
        assert_eq!(k.path.len(), 4);
        assert_eq!(k.leaf, "e");
    }
}
