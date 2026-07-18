//! Presentation-local catalog tree for the workbench sidebar.
//!
//! Domain snapshots stay in the engine; this model holds only display
//! projections and context-revision correlation for stale rejection.

use crate::effect::RequestToken;

/// Node load/error projection (text+glyph, never color alone).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CatalogNodeStatus {
    #[default]
    Ready,
    Loading,
    Stale,
    Failed,
    Unsupported,
}

impl CatalogNodeStatus {
    #[must_use]
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Ready => "",
            Self::Loading => " …",
            Self::Stale => " ~",
            Self::Failed => " !",
            Self::Unsupported => " ∅",
        }
    }
}

/// One flattened catalog row for TermRock `Tree`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogNodeProjection {
    /// Stable presentation id (path-like string, not engine CatalogNodeId).
    pub id: String,
    pub label: String,
    pub kind_label: String,
    pub depth: u16,
    pub branch: bool,
    pub expanded: bool,
    pub status: CatalogNodeStatus,
}

impl CatalogNodeProjection {
    #[must_use]
    pub fn tree_label(&self) -> String {
        format!("{} {}{}", self.kind_label, self.label, self.status.glyph())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogModel {
    Idle,
    Loading {
        request_token: RequestToken,
        context_revision: u64,
    },
    Loaded {
        request_token: RequestToken,
        context_revision: u64,
        nodes: Vec<CatalogNodeProjection>,
        selected_id: Option<String>,
        filter: String,
        truncated: bool,
    },
    Failed {
        request_token: RequestToken,
        context_revision: u64,
        reason: String,
    },
}

impl Default for CatalogModel {
    fn default() -> Self {
        Self::Idle
    }
}

impl CatalogModel {
    #[must_use]
    pub fn status_line(&self) -> String {
        match self {
            Self::Idle => "Catalog: —".into(),
            Self::Loading { .. } => "Catalog: loading…".into(),
            Self::Loaded {
                nodes,
                filter,
                truncated,
                ..
            } => {
                let visible = Self::filter_nodes(nodes, filter).len();
                let trunc = if *truncated { " trunc" } else { "" };
                if filter.is_empty() {
                    format!("Catalog: {visible}{trunc}")
                } else {
                    format!("Catalog: {visible}/{} filter{trunc}", nodes.len())
                }
            }
            Self::Failed { reason, .. } => format!("Catalog: error ({reason})"),
        }
    }

    #[must_use]
    pub const fn context_revision(&self) -> Option<u64> {
        match self {
            Self::Idle => None,
            Self::Loading {
                context_revision, ..
            }
            | Self::Loaded {
                context_revision, ..
            }
            | Self::Failed {
                context_revision, ..
            } => Some(*context_revision),
        }
    }

    #[must_use]
    pub const fn active_token(&self) -> Option<RequestToken> {
        match self {
            Self::Idle => None,
            Self::Loading { request_token, .. }
            | Self::Loaded { request_token, .. }
            | Self::Failed { request_token, .. } => Some(*request_token),
        }
    }

    /// Reject completions that do not match the live context revision.
    #[must_use]
    pub fn accepts(&self, request_token: RequestToken, context_revision: u64) -> bool {
        self.active_token() == Some(request_token)
            && self.context_revision() == Some(context_revision)
    }

    #[must_use]
    pub fn visible_nodes(&self) -> Vec<&CatalogNodeProjection> {
        match self {
            Self::Loaded { nodes, filter, .. } => Self::filter_nodes(nodes, filter),
            _ => Vec::new(),
        }
    }

    pub fn push_filter(&mut self, text: &str) {
        if let Self::Loaded {
            filter,
            selected_id,
            nodes,
            ..
        } = self
        {
            filter.push_str(text);
            let visible = Self::filter_nodes(nodes, filter);
            *selected_id = visible.first().map(|n| n.id.clone());
        }
    }

    pub fn clear_filter(&mut self) {
        if let Self::Loaded {
            filter,
            selected_id,
            nodes,
            ..
        } = self
        {
            filter.clear();
            *selected_id = nodes.first().map(|n| n.id.clone());
        }
    }

    pub fn toggle_expand(&mut self, id: &str) {
        if let Self::Loaded { nodes, .. } = self
            && let Some(node) = nodes.iter_mut().find(|n| n.id == id && n.branch)
        {
            node.expanded = !node.expanded;
        }
    }

    pub fn set_node_status(&mut self, id: &str, status: CatalogNodeStatus) {
        if let Self::Loaded { nodes, .. } = self
            && let Some(node) = nodes.iter_mut().find(|n| n.id == id)
        {
            node.status = status;
        }
    }

    /// Merge children under `parent_id` (or replace roots when parent is None).
    pub fn merge_children(
        &mut self,
        parent_id: Option<&str>,
        children: Vec<CatalogNodeProjection>,
        truncated: bool,
    ) {
        let Self::Loaded {
            nodes,
            selected_id,
            truncated: trunc_flag,
            ..
        } = self
        else {
            return;
        };
        *trunc_flag = truncated;
        match parent_id {
            None => {
                *nodes = children;
                *selected_id = nodes.first().map(|n| n.id.clone());
            }
            Some(parent) => {
                // Drop previous children of this parent (depth > parent.depth with matching prefix).
                let parent_depth = nodes
                    .iter()
                    .find(|n| n.id == parent)
                    .map(|n| n.depth)
                    .unwrap_or(0);
                if let Some(node) = nodes.iter_mut().find(|n| n.id == parent) {
                    node.expanded = true;
                    node.status = CatalogNodeStatus::Ready;
                }
                let prefix = format!("{parent}/");
                nodes.retain(|n| !(n.id.starts_with(&prefix) && n.depth > parent_depth));
                let insert_at = nodes
                    .iter()
                    .position(|n| n.id == parent)
                    .map(|i| i + 1)
                    .unwrap_or(nodes.len());
                for (offset, child) in children.into_iter().enumerate() {
                    nodes.insert(insert_at + offset, child);
                }
            }
        }
    }

    fn filter_nodes<'a>(
        nodes: &'a [CatalogNodeProjection],
        filter: &str,
    ) -> Vec<&'a CatalogNodeProjection> {
        let needle = filter.trim().to_ascii_lowercase();
        if needle.is_empty() {
            // Only show expanded branches' descendants.
            return Self::expanded_window(nodes);
        }
        // Preserve ancestors of matching leaves.
        let mut keep = vec![false; nodes.len()];
        for (index, node) in nodes.iter().enumerate() {
            if node.label.to_ascii_lowercase().contains(&needle)
                || node.kind_label.to_ascii_lowercase().contains(&needle)
            {
                keep[index] = true;
                // Mark all ancestors by depth walk backward.
                let mut depth = node.depth;
                for prev in (0..index).rev() {
                    if nodes[prev].depth < depth {
                        keep[prev] = true;
                        depth = nodes[prev].depth;
                        if depth == 0 {
                            break;
                        }
                    }
                }
            }
        }
        nodes
            .iter()
            .enumerate()
            .filter(|(i, _)| keep[*i])
            .map(|(_, n)| n)
            .collect()
    }

    fn expanded_window(nodes: &[CatalogNodeProjection]) -> Vec<&CatalogNodeProjection> {
        let mut out = Vec::new();
        let mut hide_below: Option<u16> = None;
        for node in nodes {
            if let Some(limit) = hide_below {
                if node.depth > limit {
                    continue;
                }
                hide_below = None;
            }
            out.push(node);
            if node.branch && !node.expanded {
                hide_below = Some(node.depth);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, depth: u16, branch: bool, expanded: bool) -> CatalogNodeProjection {
        CatalogNodeProjection {
            id: id.into(),
            label: id.into(),
            kind_label: "db".into(),
            depth,
            branch,
            expanded,
            status: CatalogNodeStatus::Ready,
        }
    }

    #[test]
    fn accepts_matching_token_and_revision_only() {
        let model = CatalogModel::Loading {
            request_token: 3,
            context_revision: 9,
        };
        assert!(model.accepts(3, 9));
        assert!(!model.accepts(3, 8));
        assert!(!model.accepts(2, 9));
    }

    #[test]
    fn filter_preserves_ancestors() {
        let nodes = vec![
            node("a", 0, true, true),
            node("a/b", 1, true, true),
            node("a/b/c", 2, false, false),
            node("x", 0, false, false),
        ];
        let model = CatalogModel::Loaded {
            request_token: 1,
            context_revision: 1,
            nodes,
            selected_id: None,
            filter: "c".into(),
            truncated: false,
        };
        let visible: Vec<_> = model
            .visible_nodes()
            .into_iter()
            .map(|n| n.id.as_str())
            .collect();
        assert_eq!(visible, ["a", "a/b", "a/b/c"]);
    }

    #[test]
    fn collapsed_branch_hides_descendants() {
        let nodes = vec![
            node("a", 0, true, false),
            node("a/b", 1, false, false),
            node("c", 0, false, false),
        ];
        let model = CatalogModel::Loaded {
            request_token: 1,
            context_revision: 1,
            nodes,
            selected_id: None,
            filter: String::new(),
            truncated: false,
        };
        let visible: Vec<_> = model
            .visible_nodes()
            .into_iter()
            .map(|n| n.id.as_str())
            .collect();
        assert_eq!(visible, ["a", "c"]);
    }
}
