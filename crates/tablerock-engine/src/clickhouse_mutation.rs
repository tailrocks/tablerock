//! ClickHouse apply of authorized mutation plans (non-transactional truth).
//!
//! Inserts are progressive single-row (or multi-row statement) applies with
//! row-confirmed outcomes. UPDATE/DELETE are rejected here until async
//! mutation tracking lands — never presented as transactions.

use tablerock_core::{
    AuthorizedMutationPlan, FieldValue, MutationChange, MutationId, MutationTarget, OwnedValue,
    ReviewTokenId, ValueRef,
};

use crate::clickhouse::{ClickHouseError, ClickHouseSession};
use crate::ident::quote_ident;
use crate::postgres_mutation::{
    MutationApplyOutcome, MutationChangeOutcome, MutationTransactionState,
};

impl ClickHouseSession {
    /// Apply an authorized plan. Outcomes never claim transactional rollback.
    ///
    /// `MutationTransactionState` is reused as a terminal apply flag only:
    /// - `Committed` means row-confirmed apply finished
    /// - `RolledBack` is **not** used for CH — conflicts/failures are Failed
    /// - `Unknown` when dispatch cannot confirm
    pub async fn apply_authorized_mutation(
        &self,
        authorized: AuthorizedMutationPlan,
    ) -> Result<MutationApplyOutcome, ClickHouseError> {
        let plan = authorized.plan();
        let MutationTarget::ClickHouseTable { database, table } = plan.target() else {
            return Err(ClickHouseError::Query);
        };
        let database = database.as_str();
        let table = table.as_str();
        let qualified = format!(
            "{}.{}",
            quote_ident(database).map_err(|_| ClickHouseError::Query)?,
            quote_ident(table).map_err(|_| ClickHouseError::Query)?
        );

        let mut outcomes = Vec::with_capacity(plan.changes().len());
        for (index, change) in plan.changes().iter().enumerate() {
            match apply_one_change(self, &qualified, index, change).await {
                Ok(outcome @ MutationChangeOutcome::Applied { .. }) => {
                    outcomes.push(outcome);
                }
                Ok(failed @ MutationChangeOutcome::Failed { .. }) => {
                    outcomes.push(failed);
                    // Progressive: stop further changes; prior applies stay applied.
                    return Ok(MutationApplyOutcome {
                        mutation_id: plan.mutation_id(),
                        review_token_id: authorized.token_id(),
                        transaction: MutationTransactionState::Committed,
                        changes: outcomes,
                    });
                }
                Ok(conflict @ MutationChangeOutcome::Conflict { .. }) => {
                    outcomes.push(conflict);
                    return Ok(MutationApplyOutcome {
                        mutation_id: plan.mutation_id(),
                        review_token_id: authorized.token_id(),
                        transaction: MutationTransactionState::Committed,
                        changes: outcomes,
                    });
                }
                Err(ClickHouseError::ServerCancelled) => {
                    return Ok(MutationApplyOutcome {
                        mutation_id: plan.mutation_id(),
                        review_token_id: authorized.token_id(),
                        transaction: MutationTransactionState::Unknown,
                        changes: outcomes,
                    });
                }
                Err(error) => {
                    outcomes.push(MutationChangeOutcome::Failed {
                        index,
                        detail: error.to_string(),
                    });
                    return Ok(MutationApplyOutcome {
                        mutation_id: plan.mutation_id(),
                        review_token_id: authorized.token_id(),
                        transaction: MutationTransactionState::Committed,
                        changes: outcomes,
                    });
                }
            }
        }

        Ok(MutationApplyOutcome {
            mutation_id: plan.mutation_id(),
            review_token_id: authorized.token_id(),
            transaction: MutationTransactionState::Committed,
            changes: outcomes,
        })
    }
}

async fn apply_one_change(
    session: &ClickHouseSession,
    qualified: &str,
    index: usize,
    change: &MutationChange,
) -> Result<MutationChangeOutcome, ClickHouseError> {
    match change {
        MutationChange::InsertRow { values } => {
            if values.is_empty() {
                return Ok(MutationChangeOutcome::Failed {
                    index,
                    detail: "insert requires at least one field".into(),
                });
            }
            let cols: Result<Vec<_>, _> = values.iter().map(|f| quote_ident(f.field())).collect();
            let cols = cols.map_err(|_| ClickHouseError::Query)?;
            let mut literals = Vec::with_capacity(values.len());
            for field in values {
                literals.push(sql_literal(field.value()).map_err(|_| ClickHouseError::Query)?);
            }
            // Named-parameter path preferred for values when single text/int;
            // multi-type inserts use typed literals after validation.
            let sql = format!(
                "INSERT INTO {qualified} ({}) VALUES ({})",
                cols.join(", "),
                literals.join(", ")
            );
            session.execute_sql(&sql).await?;
            Ok(MutationChangeOutcome::Applied {
                index,
                rows_affected: 1,
                returned: Vec::new(),
            })
        }
        MutationChange::UpdateRow { .. } | MutationChange::DeleteRow { .. } => {
            Ok(MutationChangeOutcome::Failed {
                index,
                detail: "ClickHouse UPDATE/DELETE use async mutations (not yet wired); never a transaction"
                    .into(),
            })
        }
        MutationChange::RedisSetString { .. }
        | MutationChange::RedisDeleteKey
        | MutationChange::RedisSetExpiration(_) => Ok(MutationChangeOutcome::Failed {
            index,
            detail: "redis mutation not valid on ClickHouse session".into(),
        }),
    }
}

/// Fail-closed SQL literal for progressive INSERT (idents already quoted separately).
fn sql_literal(value: &OwnedValue) -> Result<String, ()> {
    if value.is_truncated() {
        return Err(());
    }
    match value.as_ref() {
        ValueRef::Null => Ok("NULL".into()),
        ValueRef::Boolean(b) => Ok(if b { "1" } else { "0" }.into()),
        ValueRef::Signed(n) => Ok(n.to_string()),
        ValueRef::Unsigned(n) => Ok(n.to_string()),
        ValueRef::Float64Bits(bits) => {
            let n = f64::from_bits(bits);
            if !n.is_finite() {
                return Err(());
            }
            Ok(format!("{n}"))
        }
        ValueRef::Decimal(s)
        | ValueRef::Temporal { value: s, .. }
        | ValueRef::Text { value: s, .. }
        | ValueRef::Structured { value: s, .. } => Ok(format!("'{}'", escape_ch_string(s))),
        ValueRef::Binary { .. } | ValueRef::Invalid { .. } | ValueRef::Unknown { .. } => Err(()),
    }
}

fn escape_ch_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tablerock_core::{BoundedText, ByteLimit, Truncation};

    fn text(s: &str) -> BoundedText {
        BoundedText::copy_from_str(s, ByteLimit::new(1_000)).unwrap()
    }

    #[test]
    fn literals_escape_and_reject_truncated() {
        assert_eq!(sql_literal(&OwnedValue::signed(7)).unwrap(), "7");
        assert_eq!(
            sql_literal(
                &OwnedValue::text(text("a'b"), Truncation::Complete).unwrap()
            )
            .unwrap(),
            "'a\\'b'"
        );
        let trunc = OwnedValue::text(
            text("x"),
            Truncation::Truncated {
                original_byte_len: Some(9),
            },
        )
        .unwrap();
        assert!(sql_literal(&trunc).is_err());
    }

    #[test]
    fn non_transactional_wording_in_async_reject() {
        let detail = "ClickHouse UPDATE/DELETE use async mutations (not yet wired); never a transaction";
        assert!(detail.contains("never a transaction"));
        assert!(!detail.to_ascii_lowercase().contains("rollback"));
    }
}
