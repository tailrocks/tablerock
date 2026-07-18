//! Schema-aware SQL completion candidates (pure presentation projections).
//!
//! Ranking and filtering are pure; TermRock `CompletionMenu` only paints.
//! Stale sessions (text / context / catalog revision mismatch) never apply.

use super::catalog::{CatalogModel, CatalogNodeProjection};
use super::query_editor::QueryEditorModel;

/// One completion candidate ready for the menu + commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionCandidateView {
    pub id: String,
    pub label: String,
    pub kind: String,
    /// Byte range in editor text to replace on commit.
    pub replace_start: usize,
    pub replace_end: usize,
}

/// Open completion session tied to three revision axes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionSession {
    pub text_revision: u64,
    pub context_revision: u64,
    pub catalog_revision: u64,
    pub candidates: Vec<CompletionCandidateView>,
    pub selected_id: Option<String>,
}

impl CompletionSession {
    /// True when all three axes still match the live editor/catalog/workbench.
    #[must_use]
    pub fn is_fresh(
        &self,
        text_revision: u64,
        context_revision: u64,
        catalog_revision: u64,
    ) -> bool {
        self.text_revision == text_revision
            && self.context_revision == context_revision
            && self.catalog_revision == catalog_revision
    }
}

const KEYWORDS: &[&str] = &[
    "SELECT",
    "FROM",
    "WHERE",
    "AND",
    "OR",
    "NOT",
    "INSERT",
    "INTO",
    "VALUES",
    "UPDATE",
    "SET",
    "DELETE",
    "JOIN",
    "LEFT",
    "RIGHT",
    "INNER",
    "OUTER",
    "ON",
    "GROUP",
    "BY",
    "ORDER",
    "ASC",
    "DESC",
    "LIMIT",
    "OFFSET",
    "AS",
    "WITH",
    "UNION",
    "ALL",
    "DISTINCT",
    "HAVING",
    "CREATE",
    "TABLE",
    "VIEW",
    "INDEX",
    "DROP",
    "ALTER",
    "NULL",
    "IS",
    "IN",
    "LIKE",
    "BETWEEN",
    "EXISTS",
    "CASE",
    "WHEN",
    "THEN",
    "ELSE",
    "END",
    "TRUE",
    "FALSE",
    "RETURNING",
];

/// Catalog generation used for staleness (0 when catalog not loaded).
#[must_use]
pub fn catalog_revision(catalog: &CatalogModel) -> u64 {
    match catalog {
        CatalogModel::Loaded {
            context_revision, ..
        }
        | CatalogModel::Loading {
            context_revision, ..
        }
        | CatalogModel::Failed {
            context_revision, ..
        } => *context_revision,
        CatalogModel::Idle => 0,
    }
}

/// Build candidates for the token under the editor cursor.
#[must_use]
pub fn build_session(
    editor: &QueryEditorModel,
    catalog: &CatalogModel,
    context_revision: u64,
) -> CompletionSession {
    let (prefix, replace_start, replace_end) = token_under_cursor(editor.text(), editor.cursor());
    let prefix_upper = prefix.to_ascii_uppercase();
    let mut candidates = Vec::new();

    for kw in KEYWORDS {
        if prefix.is_empty() || kw.starts_with(&prefix_upper) {
            candidates.push(CompletionCandidateView {
                id: format!("kw:{kw}"),
                label: (*kw).to_owned(),
                kind: "keyword".into(),
                replace_start,
                replace_end,
            });
        }
    }

    if let CatalogModel::Loaded { nodes, .. } = catalog {
        for node in nodes {
            if !is_completable_object(node) {
                continue;
            }
            if prefix.is_empty()
                || node
                    .label
                    .to_ascii_lowercase()
                    .starts_with(&prefix.to_ascii_lowercase())
            {
                candidates.push(CompletionCandidateView {
                    id: format!("obj:{}", node.id),
                    label: node.label.clone(),
                    kind: node.kind_label.clone(),
                    replace_start,
                    replace_end,
                });
            }
        }
    }

    // Cap for menu height; caller ranking is prefix match order (keywords then catalog).
    candidates.truncate(64);
    let selected_id = candidates.first().map(|c| c.id.clone());
    CompletionSession {
        text_revision: editor.revision(),
        context_revision,
        catalog_revision: catalog_revision(catalog),
        candidates,
        selected_id,
    }
}

/// Redis command editor completion from the curated command table.
///
/// Provenance: Redis open-command family names (classification + completion),
/// not a vendored redis-doc JSON dump. Catalog axis is unused (revision 0).
#[must_use]
pub fn build_redis_session(editor: &QueryEditorModel, context_revision: u64) -> CompletionSession {
    use super::redis_command::{classify_command, complete_prefix};
    let (prefix, replace_start, replace_end) = token_under_cursor(editor.text(), editor.cursor());
    let hits = complete_prefix(&prefix, 64);
    let candidates: Vec<CompletionCandidateView> = hits
        .into_iter()
        .map(|name| {
            let safety = classify_command(name);
            CompletionCandidateView {
                id: format!("redis:{name}"),
                label: name.to_owned(),
                kind: format!("command/{}", safety.label()),
                replace_start,
                replace_end,
            }
        })
        .collect();
    let selected_id = candidates.first().map(|c| c.id.clone());
    CompletionSession {
        text_revision: editor.revision(),
        context_revision,
        catalog_revision: 0,
        candidates,
        selected_id,
    }
}

fn is_completable_object(node: &CatalogNodeProjection) -> bool {
    matches!(
        node.kind_label.as_str(),
        "table" | "view" | "matview" | "schema" | "database" | "column" | "function" | "ftable"
    )
}

/// Identifier token under `cursor` (letters, digits, `_`, `.` for schema.table).
fn token_under_cursor(text: &str, cursor: usize) -> (String, usize, usize) {
    let cursor = cursor.min(text.len());
    let bytes = text.as_bytes();
    let mut start = cursor;
    while start > 0 {
        let prev = {
            let mut i = start - 1;
            while i > 0 && !text.is_char_boundary(i) {
                i -= 1;
            }
            i
        };
        let ch = text[prev..start].chars().next().unwrap_or('\0');
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
            start = prev;
        } else {
            break;
        }
    }
    let mut end = cursor;
    while end < bytes.len() {
        let ch = text[end..].chars().next().unwrap_or('\0');
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
            end += ch.len_utf8();
        } else {
            break;
        }
    }
    (text[start..end].to_owned(), start, end)
}

/// Commit a candidate into the editor when the session is still fresh.
pub fn commit_candidate(
    editor: &mut QueryEditorModel,
    session: &CompletionSession,
    candidate_id: &str,
    context_revision: u64,
    catalog_revision: u64,
) -> Result<(), StaleCompletion> {
    if !session.is_fresh(editor.revision(), context_revision, catalog_revision) {
        return Err(StaleCompletion::Revisions);
    }
    let Some(candidate) = session.candidates.iter().find(|c| c.id == candidate_id) else {
        return Err(StaleCompletion::MissingCandidate);
    };
    let start = candidate.replace_start.min(editor.text().len());
    let end = candidate.replace_end.min(editor.text().len()).max(start);
    // Replace range by selecting it then inserting.
    editor.set_selection(start, end);
    editor.insert(&candidate.label);
    // Trailing space after keywords / Redis commands for continuous typing.
    if candidate.kind == "keyword" || candidate.kind.starts_with("command/") {
        editor.insert(" ");
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleCompletion {
    Revisions,
    MissingCandidate,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::catalog::CatalogNodeStatus;
    use tablerock_core::SqlDialect;

    fn editor_with(text: &str, cursor: usize) -> QueryEditorModel {
        let mut ed = QueryEditorModel::new(SqlDialect::PostgreSql);
        ed.set_text(text);
        ed.set_cursor(cursor.min(text.len()));
        ed
    }

    fn catalog_with_table(name: &str, rev: u64) -> CatalogModel {
        CatalogModel::Loaded {
            request_token: 1,
            context_revision: rev,
            nodes: vec![CatalogNodeProjection {
                id: format!("public/{name}"),
                label: name.into(),
                kind_label: "table".into(),
                depth: 2,
                branch: false,
                expanded: false,
                status: CatalogNodeStatus::Ready,
            }],
            selected_id: None,
            filter: String::new(),
            truncated: false,
        }
    }

    #[test]
    fn keywords_and_catalog_filter_by_prefix() {
        let ed = editor_with("SEL", 3);
        let cat = catalog_with_table("users", 2);
        let session = build_session(&ed, &cat, 2);
        assert!(
            session
                .candidates
                .iter()
                .any(|c| c.label == "SELECT" && c.kind == "keyword")
        );
        assert!(!session.candidates.iter().any(|c| c.label == "FROM"));
        // Catalog object matches its own prefix, not the keyword prefix.
        let ed2 = editor_with("use", 3);
        let session2 = build_session(&ed2, &cat, 2);
        assert!(session2.candidates.iter().any(|c| c.label == "users"));
    }

    #[test]
    fn stale_text_revision_rejects_commit() {
        let mut ed = editor_with("SE", 2);
        let cat = catalog_with_table("t", 1);
        let session = build_session(&ed, &cat, 1);
        // Mutate text → revision bumps.
        ed.insert("L");
        assert!(!session.is_fresh(ed.revision(), 1, 1));
        assert_eq!(
            commit_candidate(&mut ed, &session, "kw:SELECT", 1, 1),
            Err(StaleCompletion::Revisions)
        );
    }

    #[test]
    fn stale_catalog_revision_rejects_commit() {
        let mut ed = editor_with("u", 1);
        let cat = catalog_with_table("users", 3);
        let session = build_session(&ed, &cat, 3);
        assert_eq!(
            commit_candidate(&mut ed, &session, "obj:public/users", 3, 9),
            Err(StaleCompletion::Revisions)
        );
    }

    #[test]
    fn commit_replaces_token_range() {
        let mut ed = editor_with("SEL", 3);
        let cat = CatalogModel::Idle;
        let session = build_session(&ed, &cat, 1);
        let id = session
            .candidates
            .iter()
            .find(|c| c.label == "SELECT")
            .map(|c| c.id.clone())
            .expect("SELECT");
        commit_candidate(&mut ed, &session, &id, 1, 0).unwrap();
        assert!(ed.text().starts_with("SELECT"), "{}", ed.text());
    }

    #[test]
    fn redis_session_uses_command_table_not_sql_keywords() {
        let ed = editor_with("HGE", 3);
        let session = build_redis_session(&ed, 4);
        assert_eq!(session.catalog_revision, 0);
        assert_eq!(session.context_revision, 4);
        assert!(
            session
                .candidates
                .iter()
                .any(|c| c.label == "HGET" && c.kind.contains("read-only")),
            "{:?}",
            session.candidates
        );
        assert!(!session.candidates.iter().any(|c| c.label == "SELECT"));
        // Prefix that matches SQL but not Redis should not invent SQL.
        let ed2 = editor_with("SEL", 3);
        let session2 = build_redis_session(&ed2, 1);
        assert!(
            session2.candidates.is_empty()
                || !session2.candidates.iter().any(|c| c.label == "SELECT")
        );
    }

    #[test]
    fn redis_commit_inserts_command_and_space() {
        let mut ed = editor_with("HG", 2);
        let session = build_redis_session(&ed, 1);
        let id = session
            .candidates
            .iter()
            .find(|c| c.label == "HGET")
            .map(|c| c.id.clone())
            .expect("HGET");
        commit_candidate(&mut ed, &session, &id, 1, 0).unwrap();
        assert_eq!(ed.text(), "HGET ");
    }

    #[test]
    fn injection_prefix_after_quote_still_only_edits_buffer() {
        // Completing after an open quote must not execute anything — pure buffer edit.
        let mut ed = editor_with("SELECT * FROM t WHERE name = '", 32);
        let cat = catalog_with_table("evil", 1);
        let session = build_session(&ed, &cat, 1);
        // Token under cursor is empty after quote; commit a keyword is still text-only.
        if let Some(id) = session.selected_id.clone() {
            let _ = commit_candidate(&mut ed, &session, &id, 1, 1);
        }
        assert!(ed.text().contains("WHERE name = '"));
        // No side effect surface beyond text mutation.
        assert!(ed.revision() >= 1);
    }
}
