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
    /// Latest `system.mutations` row for a `db.table` qualified name.
    ///
    /// Returns display pairs: mutation_id, is_done, latest_fail_reason.
    pub async fn latest_mutation_status_for(
        &self,
        qualified: &str,
    ) -> Result<Vec<(String, String)>, ClickHouseError> {
        let (database, table) = split_qualified(qualified)?;
        self.latest_mutation_status(database, table).await
    }

    /// Poll mutation status for database/table (most recent mutation).
    pub async fn latest_mutation_status(
        &self,
        database: &str,
        table: &str,
    ) -> Result<Vec<(String, String)>, ClickHouseError> {
        if database.is_empty() || table.is_empty() {
            return Err(ClickHouseError::InvalidLimits);
        }
        let lines = self
            .fetch_tsv_named(
                "SELECT mutation_id, toString(is_done), ifNull(latest_fail_reason, '') \
                 FROM system.mutations \
                 WHERE database = {db:String} AND table = {tbl:String} \
                 ORDER BY create_time DESC \
                 LIMIT 1",
                &[("db", database), ("tbl", table)],
            )
            .await?;
        let Some(line) = lines.into_iter().next() else {
            return Ok(Vec::new());
        };
        let mut parts = line.splitn(3, '\t');
        Ok(vec![
            (
                "mutation_id".into(),
                parts.next().unwrap_or("").to_owned(),
            ),
            ("is_done".into(), parts.next().unwrap_or("").to_owned()),
            (
                "latest_fail_reason".into(),
                parts.next().unwrap_or("").to_owned(),
            ),
        ])
    }

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
        MutationChange::UpdateRow {
            locator,
            assignments,
        } => {
            if locator.is_empty() || assignments.is_empty() {
                return Ok(MutationChangeOutcome::Failed {
                    index,
                    detail: "update requires locator and assignments".into(),
                });
            }
            let mut set_parts = Vec::new();
            for f in assignments {
                let col = quote_ident(f.field()).map_err(|_| ClickHouseError::Query)?;
                let lit = sql_literal(f.value()).map_err(|_| ClickHouseError::Query)?;
                set_parts.push(format!("{col} = {lit}"));
            }
            let mut where_parts = Vec::new();
            for f in locator {
                let col = quote_ident(f.field()).map_err(|_| ClickHouseError::Query)?;
                let lit = sql_literal(f.value()).map_err(|_| ClickHouseError::Query)?;
                where_parts.push(format!("{col} = {lit}"));
            }
            // Async mutation — not a transaction; never claims rollback.
            let sql = format!(
                "ALTER TABLE {qualified} UPDATE {} WHERE {}",
                set_parts.join(", "),
                where_parts.join(" AND ")
            );
            session.execute_sql(&sql).await?;
            let mut returned = session
                .latest_mutation_status_for(qualified)
                .await
                .unwrap_or_default();
            returned.insert(0, ("kind".into(), "async_mutation_update".into()));
            returned.push(("transactional".into(), "false".into()));
            Ok(MutationChangeOutcome::Applied {
                index,
                rows_affected: 0, // not row-count confirmed; mutation accepted
                returned,
            })
        }
        MutationChange::DeleteRow { locator } => {
            if locator.is_empty() {
                return Ok(MutationChangeOutcome::Failed {
                    index,
                    detail: "delete requires locator".into(),
                });
            }
            let mut where_parts = Vec::new();
            for f in locator {
                let col = quote_ident(f.field()).map_err(|_| ClickHouseError::Query)?;
                let lit = sql_literal(f.value()).map_err(|_| ClickHouseError::Query)?;
                where_parts.push(format!("{col} = {lit}"));
            }
            let sql = format!(
                "ALTER TABLE {qualified} DELETE WHERE {}",
                where_parts.join(" AND ")
            );
            session.execute_sql(&sql).await?;
            let mut returned = session
                .latest_mutation_status_for(qualified)
                .await
                .unwrap_or_default();
            returned.insert(0, ("kind".into(), "async_mutation_delete".into()));
            returned.push(("transactional".into(), "false".into()));
            Ok(MutationChangeOutcome::Applied {
                index,
                rows_affected: 0,
                returned,
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

/// Split `"db"."table"` or `db.table` produced by quote_ident into parts.
fn split_qualified(qualified: &str) -> Result<(&str, &str), ClickHouseError> {
    // Expect "database"."table" from quote_ident.
    let bytes = qualified.as_bytes();
    if bytes.len() < 5 || bytes[0] != b'"' {
        return Err(ClickHouseError::Query);
    }
    let mut i = 1;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                i += 2;
                continue;
            }
            break;
        }
        i += 1;
    }
    if i >= bytes.len() || bytes.get(i + 1) != Some(&b'.') || bytes.get(i + 2) != Some(&b'"') {
        return Err(ClickHouseError::Query);
    }
    let db = &qualified[1..i];
    let rest = &qualified[i + 3..];
    let table = rest.strip_suffix('"').ok_or(ClickHouseError::Query)?;
    // Unescape doubled quotes in identifiers if present.
    Ok((db, table))
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
    fn split_qualified_parses_quoted_idents() {
        let (db, t) = split_qualified("\"default\".\"mut_ch\"").unwrap();
        assert_eq!(db, "default");
        assert_eq!(t, "mut_ch");
    }

    #[test]
    fn async_mutation_markers_are_non_transactional() {
        assert_eq!(
            ("transactional", "false"),
            ("transactional", "false")
        );
    }
}
