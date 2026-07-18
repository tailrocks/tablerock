//! Build typed `MutationPlan` and review previews from staged drafts.
//!
//! Execution always uses the typed plan. Preview lines are descriptive only
//! and must never be re-parsed for apply.

use tablerock_core::{
    BoundedText, ByteLimit, ContextId, FieldValue, IdParts, MutationBuildError, MutationChange,
    MutationId, MutationPlan, MutationPlanLimits, MutationTarget, OperationScope, OwnedValue,
    ProfileId, Revision, SessionId, Truncation,
};

use super::mutation_draft::{
    DraftLocatorField, MutationDraftModel, StagedCellEdit, StagedDelete, StagedInsert,
};

const TEXT_LIMIT: u64 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftPlanError {
    NotEditable,
    Empty,
    Build(MutationBuildError),
    Value(String),
}

impl std::fmt::Display for DraftPlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotEditable => f.write_str("result is not editable"),
            Self::Empty => f.write_str("no staged changes"),
            Self::Build(e) => write!(f, "mutation plan: {e}"),
            Self::Value(e) => write!(f, "value: {e}"),
        }
    }
}

/// One review line: parameterized SQL + ordered parameter display texts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewStatementLine {
    pub sql: String,
    /// Display-only parameter values in `$n` order (never used for execute).
    pub parameters: Vec<String>,
    pub kind: &'static str,
}

/// Full review projection for a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationReviewView {
    pub mutation_id_hex: String,
    pub schema: String,
    pub table: String,
    pub lines: Vec<ReviewStatementLine>,
}

fn bt(s: &str) -> Result<BoundedText, DraftPlanError> {
    BoundedText::copy_from_str(s, ByteLimit::new(TEXT_LIMIT)).map_err(|_| {
        DraftPlanError::Value(format!("text exceeds {TEXT_LIMIT} bytes"))
    })
}

/// Parse staged display text into an `OwnedValue` for plan building.
///
/// Heuristics: empty/`null` → Null; true/false → Bool; integer → Signed;
/// float → Float64Bits; otherwise complete Text.
pub fn parse_staged_value(text: &str) -> Result<OwnedValue, DraftPlanError> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
        return Ok(OwnedValue::null());
    }
    if trimmed.eq_ignore_ascii_case("true") {
        return Ok(OwnedValue::boolean(true));
    }
    if trimmed.eq_ignore_ascii_case("false") {
        return Ok(OwnedValue::boolean(false));
    }
    if let Ok(n) = trimmed.parse::<i64>() {
        return Ok(OwnedValue::signed(n));
    }
    if let Ok(n) = trimmed.parse::<f64>() {
        return Ok(OwnedValue::float64_bits(n.to_bits()));
    }
    let bound = bt(trimmed)?;
    OwnedValue::text(bound, Truncation::Complete)
        .map_err(|_| DraftPlanError::Value("invalid text value".into()))
}

fn field(name: &str, text: &str) -> Result<FieldValue, DraftPlanError> {
    Ok(FieldValue::new(bt(name)?, parse_staged_value(text)?))
}

fn locator_fields(locator: &[DraftLocatorField]) -> Result<Vec<FieldValue>, DraftPlanError> {
    locator
        .iter()
        .map(|f| field(&f.column, &f.original_text))
        .collect()
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn changes_from_drafts(drafts: &MutationDraftModel) -> Result<Vec<MutationChange>, DraftPlanError> {
    let mut changes = Vec::new();
    for insert in &drafts.inserts {
        changes.push(insert_change(insert)?);
    }
    // Group cell edits by abs_row → one UpdateRow per row.
    let mut rows: Vec<u64> = drafts.cell_edits.iter().map(|e| e.abs_row).collect();
    rows.sort_unstable();
    rows.dedup();
    for row in rows {
        let row_edits: Vec<&StagedCellEdit> = drafts
            .cell_edits
            .iter()
            .filter(|e| e.abs_row == row)
            .collect();
        if row_edits.is_empty() {
            continue;
        }
        let locator = locator_fields(&row_edits[0].locator)?;
        let assignments: Result<Vec<_>, _> = row_edits
            .iter()
            .map(|e| field(&e.column, &e.staged_text))
            .collect();
        changes.push(MutationChange::UpdateRow {
            locator,
            assignments: assignments?,
        });
    }
    for delete in &drafts.deletes {
        changes.push(delete_change(delete)?);
    }
    Ok(changes)
}

fn insert_change(insert: &StagedInsert) -> Result<MutationChange, DraftPlanError> {
    let values: Result<Vec<_>, _> = insert
        .values
        .iter()
        .map(|(col, text)| field(col, text))
        .collect();
    Ok(MutationChange::InsertRow {
        values: values?,
    })
}

fn delete_change(delete: &StagedDelete) -> Result<MutationChange, DraftPlanError> {
    Ok(MutationChange::DeleteRow {
        locator: locator_fields(&delete.locator)?,
    })
}

/// Build a typed plan from drafts + editable identity.
pub fn plan_from_drafts(
    drafts: &MutationDraftModel,
    schema: &str,
    table: &str,
    database: &str,
    profile_seed: u64,
    session_seed: u64,
    context_seed: u64,
    mutation_seed: u64,
    revision: Revision,
) -> Result<MutationPlan, DraftPlanError> {
    if !drafts.staging_allowed() {
        return Err(DraftPlanError::NotEditable);
    }
    if drafts.is_empty() {
        return Err(DraftPlanError::Empty);
    }
    let changes = changes_from_drafts(drafts)?;
    let limits = MutationPlanLimits::new(256, 64, 256 * 1024, 1024 * 1024, 60_000)
        .map_err(DraftPlanError::Build)?;
    let scope = OperationScope::new(
        ProfileId::from_parts(IdParts::new(1, profile_seed).unwrap()).unwrap(),
        SessionId::from_parts(IdParts::new(1, session_seed).unwrap()).unwrap(),
        ContextId::from_parts(IdParts::new(1, context_seed).unwrap()).unwrap(),
    );
    let target = MutationTarget::PostgreSqlRelation {
        database: bt(database)?,
        schema: bt(schema)?,
        relation: bt(table)?,
    };
    MutationPlan::new(
        MutationId::from_parts(IdParts::new(1, mutation_seed).unwrap()).unwrap(),
        scope,
        revision,
        target,
        changes,
        limits,
    )
    .map_err(DraftPlanError::Build)
}

/// Descriptive review lines from a typed plan (never executed).
pub fn review_view_from_plan(plan: &MutationPlan) -> MutationReviewView {
    let (schema, table) = match plan.target() {
        MutationTarget::PostgreSqlRelation {
            schema, relation, ..
        } => (schema.as_str().to_owned(), relation.as_str().to_owned()),
        other => (
            String::new(),
            format!("{:?}", other.engine()),
        ),
    };
    let qualified = format!("{}.{}", quote_ident(&schema), quote_ident(&table));
    let mut lines = Vec::new();
    for change in plan.changes() {
        lines.push(preview_change(&qualified, change));
    }
    MutationReviewView {
        mutation_id_hex: format!("{:?}", plan.mutation_id()),
        schema,
        table,
        lines,
    }
}

fn preview_change(qualified: &str, change: &MutationChange) -> ReviewStatementLine {
    match change {
        MutationChange::InsertRow { values } => {
            let cols: Vec<_> = values
                .iter()
                .map(|f| quote_ident(f.field()))
                .collect();
            let placeholders: Vec<_> = (1..=values.len()).map(|n| format!("${n}")).collect();
            let sql = format!(
                "INSERT INTO {qualified} ({}) VALUES ({})",
                cols.join(", "),
                placeholders.join(", ")
            );
            ReviewStatementLine {
                sql,
                parameters: values.iter().map(display_field).collect(),
                kind: "insert",
            }
        }
        MutationChange::UpdateRow {
            locator,
            assignments,
        } => {
            let set_parts: Vec<_> = assignments
                .iter()
                .enumerate()
                .map(|(i, f)| format!("{} = ${}", quote_ident(f.field()), i + 1))
                .collect();
            let where_start = assignments.len() + 1;
            let where_parts: Vec<_> = locator
                .iter()
                .enumerate()
                .map(|(i, f)| format!("{} = ${}", quote_ident(f.field()), where_start + i))
                .collect();
            let sql = format!(
                "UPDATE {qualified} SET {} WHERE {}",
                set_parts.join(", "),
                where_parts.join(" AND ")
            );
            let mut parameters: Vec<_> = assignments.iter().map(display_field).collect();
            parameters.extend(locator.iter().map(display_field));
            ReviewStatementLine {
                sql,
                parameters,
                kind: "update",
            }
        }
        MutationChange::DeleteRow { locator } => {
            let where_parts: Vec<_> = locator
                .iter()
                .enumerate()
                .map(|(i, f)| format!("{} = ${}", quote_ident(f.field()), i + 1))
                .collect();
            let sql = format!(
                "DELETE FROM {qualified} WHERE {}",
                where_parts.join(" AND ")
            );
            ReviewStatementLine {
                sql,
                parameters: locator.iter().map(display_field).collect(),
                kind: "delete",
            }
        }
        MutationChange::RedisSetString { .. } => ReviewStatementLine {
            sql: "REDIS SET …".into(),
            parameters: vec![],
            kind: "redis_set",
        },
        MutationChange::RedisDeleteKey => ReviewStatementLine {
            sql: "REDIS DEL …".into(),
            parameters: vec![],
            kind: "redis_del",
        },
        MutationChange::RedisSetExpiration(_) => ReviewStatementLine {
            sql: "REDIS EXPIRE …".into(),
            parameters: vec![],
            kind: "redis_expire",
        },
        MutationChange::RedisHashSetField { .. } => ReviewStatementLine {
            sql: "REDIS HSET …".into(),
            parameters: vec![],
            kind: "redis_hset",
        },
        MutationChange::RedisHashDeleteField { .. } => ReviewStatementLine {
            sql: "REDIS HDEL …".into(),
            parameters: vec![],
            kind: "redis_hdel",
        },
        MutationChange::RedisSetAddMember { .. } => ReviewStatementLine {
            sql: "REDIS SADD …".into(),
            parameters: vec![],
            kind: "redis_sadd",
        },
        MutationChange::RedisSetRemoveMember { .. } => ReviewStatementLine {
            sql: "REDIS SREM …".into(),
            parameters: vec![],
            kind: "redis_srem",
        },
        MutationChange::RedisZSetAddMember { .. } => ReviewStatementLine {
            sql: "REDIS ZADD …".into(),
            parameters: vec![],
            kind: "redis_zadd",
        },
        MutationChange::RedisZSetRemoveMember { .. } => ReviewStatementLine {
            sql: "REDIS ZREM …".into(),
            parameters: vec![],
            kind: "redis_zrem",
        },
    }
}

fn display_field(field: &FieldValue) -> String {
    // Kind + bounded byte size only — never dump large binary as text.
    let value = field.value();
    format!(
        "{} ({:?}, {} B{})",
        field.field(),
        value.kind(),
        value.encoded_byte_len(),
        if value.is_truncated() {
            ", truncated"
        } else {
            ""
        }
    )
}

/// Build review view directly from drafts (plan intermediate for tests).
pub fn review_from_drafts(
    drafts: &MutationDraftModel,
    schema: &str,
    table: &str,
    database: &str,
) -> Result<MutationReviewView, DraftPlanError> {
    let plan = plan_from_drafts(drafts, schema, table, database, 1, 2, 3, 10, Revision::INITIAL)?;
    Ok(review_view_from_plan(&plan))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::mutation_draft::{DraftLocatorField, MutationDraftModel, StagedCellEdit};
    use tablerock_core::{EditabilityFacts, ProfileSafetyMode};

    fn ready_draft() -> MutationDraftModel {
        let mut d = MutationDraftModel::new();
        let facts = EditabilityFacts::classify(
            ProfileSafetyMode::ConfirmWrites,
            false,
            Some("public"),
            Some("users"),
            &["id".into()],
        );
        d.apply_editability(&facts);
        d
    }

    #[test]
    fn parse_value_heuristics() {
        assert!(matches!(
            parse_staged_value("null").unwrap().as_ref(),
            tablerock_core::ValueRef::Null
        ));
        assert!(matches!(
            parse_staged_value("42").unwrap().as_ref(),
            tablerock_core::ValueRef::Signed(42)
        ));
        assert!(matches!(
            parse_staged_value("true").unwrap().as_ref(),
            tablerock_core::ValueRef::Boolean(true)
        ));
    }

    #[test]
    fn plan_and_preview_from_update() {
        let mut draft = ready_draft();
        assert!(draft.stage_cell_edit(StagedCellEdit {
            abs_row: 0,
            column: "name".into(),
            original_text: "alice".into(),
            staged_text: "bob".into(),
            locator: vec![DraftLocatorField {
                column: "id".into(),
                original_text: "1".into(),
            }],
        }));
        let plan = plan_from_drafts(
            &draft,
            "public",
            "users",
            "postgres",
            1,
            2,
            3,
            10,
            Revision::INITIAL,
        )
        .unwrap();
        assert_eq!(plan.changes().len(), 1);
        let view = review_view_from_plan(&plan);
        assert_eq!(view.lines.len(), 1);
        assert_eq!(view.lines[0].kind, "update");
        assert!(view.lines[0].sql.contains("UPDATE"));
        assert!(view.lines[0].sql.contains("$1"));
        assert!(view.lines[0].sql.contains("$2"));
        // Preview never embeds raw staged text as executable SQL values.
        assert!(!view.lines[0].sql.contains("bob"));
        assert_eq!(view.lines[0].parameters.len(), 2);
    }

    #[test]
    fn empty_and_blocked_fail_closed() {
        let draft = MutationDraftModel::new();
        assert!(matches!(
            plan_from_drafts(&draft, "public", "t", "db", 1, 2, 3, 4, Revision::INITIAL),
            Err(DraftPlanError::NotEditable)
        ));
        let mut ready = ready_draft();
        assert!(matches!(
            plan_from_drafts(&ready, "public", "t", "db", 1, 2, 3, 4, Revision::INITIAL),
            Err(DraftPlanError::Empty)
        ));
        let _ = ready;
    }

    #[test]
    fn insert_delete_preview_kinds() {
        let mut draft = ready_draft();
        draft
            .stage_insert(vec![
                ("id".into(), "9".into()),
                ("name".into(), "z".into()),
            ])
            .unwrap();
        draft.stage_delete(super::super::mutation_draft::StagedDelete {
            abs_row: 3,
            locator: vec![DraftLocatorField {
                column: "id".into(),
                original_text: "3".into(),
            }],
        });
        let view = review_from_drafts(&draft, "public", "users", "postgres").unwrap();
        assert!(view.lines.iter().any(|l| l.kind == "insert"));
        assert!(view.lines.iter().any(|l| l.kind == "delete"));
    }
}
