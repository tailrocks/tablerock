//! PostgreSQL apply of authorized mutation plans (transactional, handle-based).
//!
//! SQL is built only from quoted identifiers + `$n` parameters. Preview text
//! is never re-parsed or executed.

use tablerock_core::{
    AuthorizedMutationPlan, FieldValue, MutationChange, MutationId, MutationTarget, OwnedValue,
    ReviewTokenId, ValueRef,
};
use tokio_postgres::types::ToSql;

use crate::ident::quote_ident;
use crate::postgres::{PostgresError, PostgresSession};

/// Per-change outcome inside a single apply attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationChangeOutcome {
    Applied {
        index: usize,
        rows_affected: u64,
    },
    Conflict {
        index: usize,
        rows_affected: u64,
        detail: &'static str,
    },
    Failed {
        index: usize,
        detail: String,
    },
}

/// Terminal state of the apply transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationTransactionState {
    Committed,
    RolledBack,
    /// Dispatched write with no confirmed terminal state — never retried.
    Unknown,
}

/// Typed outcome of applying an authorized plan (never plan bytes on the wire).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationApplyOutcome {
    pub mutation_id: MutationId,
    pub review_token_id: ReviewTokenId,
    pub transaction: MutationTransactionState,
    pub changes: Vec<MutationChangeOutcome>,
}

impl PostgresSession {
    /// Apply an authorized mutation plan in one transaction.
    ///
    /// Affected-row count must be exactly 1 for update/delete of a single row.
    /// Any other count rolls the transaction back and reports conflict.
    pub async fn apply_authorized_mutation(
        &self,
        authorized: AuthorizedMutationPlan,
    ) -> Result<MutationApplyOutcome, PostgresError> {
        let plan = authorized.plan();
        let MutationTarget::PostgreSqlRelation {
            database: _,
            schema,
            relation,
        } = plan.target()
        else {
            return Err(PostgresError::Query);
        };
        let schema = schema.as_str();
        let relation = relation.as_str();
        let qualified = format!(
            "{}.{}",
            quote_ident(schema).map_err(|_| PostgresError::Query)?,
            quote_ident(relation).map_err(|_| PostgresError::Query)?
        );

        // BEGIN/COMMIT via Client (not Clone); explicit SQL keeps Arc session usable.
        self.client
            .batch_execute("BEGIN")
            .await
            .map_err(|_| PostgresError::Query)?;

        let mut outcomes = Vec::with_capacity(plan.changes().len());
        for (index, change) in plan.changes().iter().enumerate() {
            match apply_one_change(self, &qualified, index, change).await {
                Ok(outcome @ MutationChangeOutcome::Applied { .. }) => {
                    outcomes.push(outcome);
                }
                Ok(conflict @ MutationChangeOutcome::Conflict { .. }) => {
                    let _ = self.client.batch_execute("ROLLBACK").await;
                    outcomes.push(conflict);
                    return Ok(MutationApplyOutcome {
                        mutation_id: plan.mutation_id(),
                        review_token_id: authorized.token_id(),
                        transaction: MutationTransactionState::RolledBack,
                        changes: outcomes,
                    });
                }
                Ok(failed @ MutationChangeOutcome::Failed { .. }) => {
                    let _ = self.client.batch_execute("ROLLBACK").await;
                    outcomes.push(failed);
                    return Ok(MutationApplyOutcome {
                        mutation_id: plan.mutation_id(),
                        review_token_id: authorized.token_id(),
                        transaction: MutationTransactionState::RolledBack,
                        changes: outcomes,
                    });
                }
                Err(PostgresError::ServerCancelled) => {
                    let _ = self.client.batch_execute("ROLLBACK").await;
                    return Ok(MutationApplyOutcome {
                        mutation_id: plan.mutation_id(),
                        review_token_id: authorized.token_id(),
                        transaction: MutationTransactionState::Unknown,
                        changes: outcomes,
                    });
                }
                Err(error) => {
                    let _ = self.client.batch_execute("ROLLBACK").await;
                    outcomes.push(MutationChangeOutcome::Failed {
                        index,
                        detail: error.to_string(),
                    });
                    return Ok(MutationApplyOutcome {
                        mutation_id: plan.mutation_id(),
                        review_token_id: authorized.token_id(),
                        transaction: MutationTransactionState::RolledBack,
                        changes: outcomes,
                    });
                }
            }
        }

        match self.client.batch_execute("COMMIT").await {
            Ok(()) => Ok(MutationApplyOutcome {
                mutation_id: plan.mutation_id(),
                review_token_id: authorized.token_id(),
                transaction: MutationTransactionState::Committed,
                changes: outcomes,
            }),
            Err(_) => Ok(MutationApplyOutcome {
                mutation_id: plan.mutation_id(),
                review_token_id: authorized.token_id(),
                transaction: MutationTransactionState::Unknown,
                changes: outcomes,
            }),
        }
    }
}

async fn apply_one_change(
    session: &PostgresSession,
    qualified: &str,
    index: usize,
    change: &MutationChange,
) -> Result<MutationChangeOutcome, PostgresError> {
    match change {
        MutationChange::InsertRow { values } => {
            if values.is_empty() {
                return Ok(MutationChangeOutcome::Failed {
                    index,
                    detail: "insert requires at least one field".into(),
                });
            }
            let cols: Result<Vec<_>, _> = values
                .iter()
                .map(|f| quote_ident(f.field()))
                .collect();
            let cols = cols.map_err(|_| PostgresError::Query)?;
            let params = bind_fields(values)?;
            // Cast placeholders to the wire type we send so prepare inference
            // cannot demand INT4 for an i64 bind (int column + bigint param).
            let placeholders: Vec<_> = params
                .iter()
                .enumerate()
                .map(|(i, p)| sql_placeholder(i + 1, p))
                .collect();
            let sql = format!(
                "INSERT INTO {qualified} ({}) VALUES ({})",
                cols.join(", "),
                placeholders.join(", ")
            );
            let rows = execute_bound(session, &sql, &params).await?;
            if rows == 1 {
                Ok(MutationChangeOutcome::Applied {
                    index,
                    rows_affected: rows,
                })
            } else {
                Ok(MutationChangeOutcome::Conflict {
                    index,
                    rows_affected: rows,
                    detail: "insert affected unexpected row count",
                })
            }
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
            let mut params = bind_fields(assignments)?;
            let set_parts: Result<Vec<_>, _> = assignments
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let bound = &params[i];
                    quote_ident(f.field())
                        .map(|c| format!("{c} = {}", sql_placeholder(i + 1, bound)))
                })
                .collect();
            let set_parts = set_parts.map_err(|_| PostgresError::Query)?;
            let where_start = params.len() + 1;
            let locator_params = bind_fields(locator)?;
            let where_parts: Result<Vec<_>, _> = locator
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let bound = &locator_params[i];
                    quote_ident(f.field()).map(|c| {
                        format!("{c} = {}", sql_placeholder(where_start + i, bound))
                    })
                })
                .collect();
            let where_parts = where_parts.map_err(|_| PostgresError::Query)?;
            params.extend(locator_params);
            let sql = format!(
                "UPDATE {qualified} SET {} WHERE {}",
                set_parts.join(", "),
                where_parts.join(" AND ")
            );
            let rows = execute_bound(session, &sql, &params).await?;
            if rows == 1 {
                Ok(MutationChangeOutcome::Applied {
                    index,
                    rows_affected: rows,
                })
            } else {
                Ok(MutationChangeOutcome::Conflict {
                    index,
                    rows_affected: rows,
                    detail: "update must affect exactly one row",
                })
            }
        }
        MutationChange::DeleteRow { locator } => {
            if locator.is_empty() {
                return Ok(MutationChangeOutcome::Failed {
                    index,
                    detail: "delete requires locator".into(),
                });
            }
            let params = bind_fields(locator)?;
            let where_parts: Result<Vec<_>, _> = locator
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let bound = &params[i];
                    quote_ident(f.field())
                        .map(|c| format!("{c} = {}", sql_placeholder(i + 1, bound)))
                })
                .collect();
            let where_parts = where_parts.map_err(|_| PostgresError::Query)?;
            let sql = format!(
                "DELETE FROM {qualified} WHERE {}",
                where_parts.join(" AND ")
            );
            let rows = execute_bound(session, &sql, &params).await?;
            if rows == 1 {
                Ok(MutationChangeOutcome::Applied {
                    index,
                    rows_affected: rows,
                })
            } else {
                Ok(MutationChangeOutcome::Conflict {
                    index,
                    rows_affected: rows,
                    detail: "delete must affect exactly one row",
                })
            }
        }
        MutationChange::RedisSetString { .. }
        | MutationChange::RedisDeleteKey
        | MutationChange::RedisSetExpiration(_) => Ok(MutationChangeOutcome::Failed {
            index,
            detail: "redis mutation not valid on PostgreSQL session".into(),
        }),
    }
}

enum Bound {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    Text(String),
    Bytes(Vec<u8>),
}

/// `$n` plus a cast matching the rustls/tokio-postgres wire type we bind.
/// Without this, prepare infers INT4 from the column and rejects an i64 bind.
fn sql_placeholder(n: usize, bound: &Bound) -> String {
    match bound {
        Bound::Null => format!("${n}::text"),
        Bound::Bool(_) => format!("${n}::boolean"),
        Bound::I64(_) | Bound::U64(_) => format!("${n}::bigint"),
        Bound::F64(_) => format!("${n}::double precision"),
        Bound::Text(_) => format!("${n}::text"),
        Bound::Bytes(_) => format!("${n}::bytea"),
    }
}

fn bind_fields(fields: &[FieldValue]) -> Result<Vec<Bound>, PostgresError> {
    fields.iter().map(|f| bind_value(f.value())).collect()
}

fn bind_value(value: &OwnedValue) -> Result<Bound, PostgresError> {
    // Truncated/invalid/unknown values are not writable.
    if value.is_truncated() {
        return Err(PostgresError::Query);
    }
    match value.as_ref() {
        ValueRef::Null => Ok(Bound::Null),
        ValueRef::Boolean(b) => Ok(Bound::Bool(b)),
        ValueRef::Signed(n) => Ok(Bound::I64(n)),
        ValueRef::Unsigned(n) => Ok(Bound::U64(n)),
        ValueRef::Float64Bits(bits) => Ok(Bound::F64(f64::from_bits(bits))),
        ValueRef::Decimal(s)
        | ValueRef::Temporal { value: s, .. }
        | ValueRef::Text { value: s, .. }
        | ValueRef::Structured { value: s, .. } => Ok(Bound::Text(s.to_owned())),
        ValueRef::Binary { value: b, .. } => Ok(Bound::Bytes(b.to_vec())),
        ValueRef::Invalid { .. } | ValueRef::Unknown { .. } => Err(PostgresError::Query),
    }
}

async fn execute_bound(
    session: &PostgresSession,
    sql: &str,
    params: &[Bound],
) -> Result<u64, PostgresError> {
    let owned: Vec<Box<dyn ToSql + Sync + Send>> = params
        .iter()
        .map(|p| -> Box<dyn ToSql + Sync + Send> {
            match p {
                // Typed null matches sql_placeholder(::text).
                Bound::Null => Box::new(Option::<String>::None),
                Bound::Bool(b) => Box::new(*b),
                Bound::I64(n) => Box::new(*n),
                Bound::U64(n) => Box::new(*n as i64),
                Bound::F64(n) => Box::new(*n),
                Bound::Text(s) => Box::new(s.clone()),
                Bound::Bytes(b) => Box::new(b.clone()),
            }
        })
        .collect();
    let refs: Vec<&(dyn ToSql + Sync)> = owned
        .iter()
        .map(|p| p.as_ref() as &(dyn ToSql + Sync))
        .collect();
    session
        .client
        .execute(sql, &refs[..])
        .await
        .map_err(|_| PostgresError::Query)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tablerock_core::{
        BoundedText, ByteLimit, IdParts, MutationChange, MutationExecutionModel,
        MutationPlanLimits, MutationTarget, OwnedValue, ReviewTokenId, Truncation,
    };

    fn text(s: &str) -> BoundedText {
        BoundedText::copy_from_str(s, ByteLimit::new(10_000)).unwrap()
    }

    #[test]
    fn bind_rejects_truncated_text() {
        let value = OwnedValue::text(
            text("hello"),
            Truncation::Truncated {
                original_byte_len: Some(100),
            },
        )
        .unwrap();
        assert!(bind_value(&value).is_err());
    }

    #[test]
    fn bind_null_and_signed() {
        assert!(matches!(
            bind_value(&OwnedValue::null()).unwrap(),
            Bound::Null
        ));
        assert!(matches!(
            bind_value(&OwnedValue::signed(42)).unwrap(),
            Bound::I64(42)
        ));
    }

    #[test]
    fn mutation_change_debug_omits_values() {
        let change = MutationChange::UpdateRow {
            locator: vec![FieldValue::new(text("id"), OwnedValue::signed(1))],
            assignments: vec![FieldValue::new(
                text("name"),
                OwnedValue::text(text("secret"), Truncation::Complete).unwrap(),
            )],
        };
        let dbg = format!("{change:?}");
        assert!(!dbg.contains("secret"));
        assert!(dbg.contains("update_row"));
    }

    #[test]
    fn apply_mutations_intent_is_may_write() {
        use tablerock_core::{CommandIntent, CommandSafety};
        let token = ReviewTokenId::from_parts(IdParts::new(1, 9).unwrap()).unwrap();
        let intent = CommandIntent::ApplyMutations {
            review_token_id: token,
        };
        assert_eq!(intent.safety(), CommandSafety::MayWrite);
        let _ = MutationExecutionModel::PostgreSqlAtomicTransaction;
        let _ = MutationPlanLimits::new(8, 16, 1024, 1024, 60_000);
        let _ = MutationTarget::PostgreSqlRelation {
            database: text("postgres"),
            schema: text("public"),
            relation: text("t"),
        };
    }

    #[test]
    fn sql_placeholder_casts_match_wire_types() {
        assert_eq!(sql_placeholder(1, &Bound::I64(1)), "$1::bigint");
        assert_eq!(sql_placeholder(2, &Bound::Text("x".into())), "$2::text");
        assert_eq!(sql_placeholder(3, &Bound::Bool(true)), "$3::boolean");
        assert_eq!(sql_placeholder(4, &Bound::Null), "$4::text");
    }

    #[test]
    fn hostile_identifiers_are_quoted_not_injected() {
        // Quote-ident doubles internal quotes so the token stays one identifier.
        let hostile = "users\"; DROP TABLE t; --";
        let quoted = quote_ident(hostile).unwrap();
        assert_eq!(quoted, "\"users\"\"; DROP TABLE t; --\"");
        // Values never enter SQL as literals — only $n casts.
        let ph = sql_placeholder(1, &Bound::Text("1; DROP TABLE t".into()));
        assert_eq!(ph, "$1::text");
        assert!(!ph.contains("DROP"));
    }
}
