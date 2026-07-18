//! Apply a parsed CSV table as authorized insert mutations.
//!
//! Builds typed `MutationChange::InsertRow` values only — never SQL string
//! concatenation. Authorization is consume-once via the core review registry.

use std::sync::Arc;

use tablerock_core::{
    MutationId, MutationPlan, MutationPlanLimits, MutationReviewRegistry, MutationTarget,
    OperationScope, ReviewTokenId, Revision,
};
use tablerock_engine::{DriverSession, MutationApplyOutcome};

use crate::import_csv::{
    CsvImportError, CsvTable, csv_to_insert_changes, validate_insert_batch_size,
};

#[derive(Debug)]
pub enum ImportApplyError {
    Csv(CsvImportError),
    Plan(String),
    Review(String),
    Apply(String),
}

impl std::fmt::Display for ImportApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Csv(e) => write!(f, "{e}"),
            Self::Plan(e) | Self::Review(e) | Self::Apply(e) => f.write_str(e),
        }
    }
}

impl std::error::Error for ImportApplyError {}

/// Build a reviewed, authorized insert plan from CSV and apply it once.
///
/// `now_ms` is the wall clock used for review issue/expiry. The review token is
/// minted, inserted, and consumed inside this function — callers never see plan
/// bytes and cannot retry the same handle after a terminal outcome.
pub async fn apply_csv_inserts(
    session: Arc<dyn DriverSession>,
    table: &CsvTable,
    target: MutationTarget,
    scope: OperationScope,
    revision: Revision,
    mutation_id: MutationId,
    review_token_id: ReviewTokenId,
    max_cell_bytes: u64,
    max_changes: u32,
    now_ms: u64,
    review_validity_ms: u64,
) -> Result<MutationApplyOutcome, ImportApplyError> {
    let changes = csv_to_insert_changes(table, max_cell_bytes).map_err(ImportApplyError::Csv)?;
    validate_insert_batch_size(&changes, max_changes)
        .map_err(|e| ImportApplyError::Plan(e.to_string()))?;
    let limits = MutationPlanLimits::new(
        max_changes,
        64,
        64 * 1024,
        4 * 1024 * 1024,
        review_validity_ms,
    )
    .map_err(|e| ImportApplyError::Plan(e.to_string()))?;
    let plan = MutationPlan::new(mutation_id, scope, revision, target, changes, limits)
        .map_err(|e| ImportApplyError::Plan(e.to_string()))?;
    let expires = now_ms
        .checked_add(review_validity_ms)
        .ok_or_else(|| ImportApplyError::Review("review expiry overflow".into()))?;
    let reviewed = plan
        .review(review_token_id, now_ms, expires)
        .map_err(|e| ImportApplyError::Review(e.to_string()))?;
    let mut registry =
        MutationReviewRegistry::new(16).map_err(|e| ImportApplyError::Review(e.to_string()))?;
    registry
        .insert(reviewed, now_ms)
        .map_err(|e| ImportApplyError::Review(e.to_string()))?;
    let authorized = registry
        .authorize(review_token_id, now_ms.saturating_add(1), scope, revision)
        .map_err(|e| ImportApplyError::Review(e.to_string()))?;
    session
        .apply_authorized_mutation(authorized)
        .await
        .map_err(|e| ImportApplyError::Apply(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_csv::parse_csv;

    #[test]
    fn rejects_empty_csv_before_any_apply() {
        let table = CsvTable {
            headers: vec!["a".into()],
            rows: vec![],
        };
        let changes = csv_to_insert_changes(&table, 64).unwrap();
        assert!(changes.is_empty());
        assert!(validate_insert_batch_size(&changes, 16).is_err());
    }

    #[test]
    fn parses_multi_row_csv_for_apply_batch() {
        let table = parse_csv("id,name\n1,a\n2,b\n", 10, 64).unwrap();
        let changes = csv_to_insert_changes(&table, 64).unwrap();
        assert_eq!(changes.len(), 2);
        validate_insert_batch_size(&changes, 16).unwrap();
    }
}
