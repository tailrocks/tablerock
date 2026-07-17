use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use crate::{
    BoundedBytes, BoundedText, Engine, MutationId, OperationScope, OwnedValue, ReviewTokenId,
    Revision, ValueKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MutationExecutionModel {
    PostgreSqlAtomicTransaction,
    ClickHouseProgressiveInsertNonTransactional,
    ClickHouseAsynchronousMutationNonTransactional,
    RedisSequentialNoRollback,
}

#[derive(Clone, PartialEq, Eq)]
pub enum MutationTarget {
    PostgreSqlRelation {
        database: BoundedText,
        schema: BoundedText,
        relation: BoundedText,
    },
    ClickHouseTable {
        database: BoundedText,
        table: BoundedText,
    },
    RedisKey {
        logical_database: u32,
        key: BoundedBytes,
    },
}

impl MutationTarget {
    #[must_use]
    pub const fn engine(&self) -> Engine {
        match self {
            Self::PostgreSqlRelation { .. } => Engine::PostgreSql,
            Self::ClickHouseTable { .. } => Engine::ClickHouse,
            Self::RedisKey { .. } => Engine::Redis,
        }
    }

    fn byte_len(&self) -> u64 {
        match self {
            Self::PostgreSqlRelation {
                database,
                schema,
                relation,
            } => [database.len(), schema.len(), relation.len()]
                .into_iter()
                .map(portable_len)
                .fold(0, u64::saturating_add),
            Self::ClickHouseTable { database, table } => [database.len(), table.len()]
                .into_iter()
                .map(portable_len)
                .fold(0, u64::saturating_add),
            Self::RedisKey { key, .. } => portable_len(key.len()),
        }
    }

    fn has_empty_part(&self) -> bool {
        match self {
            Self::PostgreSqlRelation {
                database,
                schema,
                relation,
            } => database.is_empty() || schema.is_empty() || relation.is_empty(),
            Self::ClickHouseTable { database, table } => database.is_empty() || table.is_empty(),
            Self::RedisKey { key, .. } => key.is_empty(),
        }
    }
}

impl fmt::Debug for MutationTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MutationTarget")
            .field("engine", &self.engine())
            .field("identifier_bytes", &self.byte_len())
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct FieldValue {
    field: BoundedText,
    value: OwnedValue,
}

impl FieldValue {
    #[must_use]
    pub const fn new(field: BoundedText, value: OwnedValue) -> Self {
        Self { field, value }
    }

    #[must_use]
    pub fn field(&self) -> &str {
        self.field.as_str()
    }

    #[must_use]
    pub const fn value(&self) -> &OwnedValue {
        &self.value
    }
}

impl fmt::Debug for FieldValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FieldValue")
            .field("field_bytes", &self.field.len())
            .field("value_kind", &self.value.kind())
            .field("value_bytes", &self.value.encoded_byte_len())
            .field("truncated", &self.value.is_truncated())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedisExpiration {
    Preserve,
    Persist,
    ExpireAfterMillis(u64),
}

#[derive(Clone, PartialEq, Eq)]
pub enum MutationChange {
    InsertRow {
        values: Vec<FieldValue>,
    },
    UpdateRow {
        locator: Vec<FieldValue>,
        assignments: Vec<FieldValue>,
    },
    DeleteRow {
        locator: Vec<FieldValue>,
    },
    RedisSetString {
        value: BoundedBytes,
        expiration: RedisExpiration,
    },
    RedisDeleteKey,
    RedisSetExpiration(RedisExpiration),
}

impl fmt::Debug for MutationChange {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("MutationChange");
        match self {
            Self::InsertRow { values } => {
                debug
                    .field("kind", &"insert_row")
                    .field("fields", &values.len());
            }
            Self::UpdateRow {
                locator,
                assignments,
            } => {
                debug
                    .field("kind", &"update_row")
                    .field("locator_fields", &locator.len())
                    .field("assignment_fields", &assignments.len());
            }
            Self::DeleteRow { locator } => {
                debug
                    .field("kind", &"delete_row")
                    .field("fields", &locator.len());
            }
            Self::RedisSetString { value, expiration } => {
                debug
                    .field("kind", &"redis_set_string")
                    .field("value_bytes", &value.len())
                    .field("expiration", expiration);
            }
            Self::RedisDeleteKey => {
                debug.field("kind", &"redis_delete_key");
            }
            Self::RedisSetExpiration(expiration) => {
                debug
                    .field("kind", &"redis_set_expiration")
                    .field("expiration", expiration);
            }
        }
        debug.finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MutationPlanLimits {
    max_changes: u32,
    max_fields_per_change: u32,
    max_text_bytes: u64,
    max_value_bytes: u64,
    max_review_validity_ms: u64,
}

impl MutationPlanLimits {
    pub const fn new(
        max_changes: u32,
        max_fields_per_change: u32,
        max_text_bytes: u64,
        max_value_bytes: u64,
        max_review_validity_ms: u64,
    ) -> Result<Self, MutationBuildError> {
        if max_changes == 0
            || max_fields_per_change == 0
            || max_text_bytes == 0
            || max_value_bytes == 0
            || max_review_validity_ms == 0
        {
            return Err(MutationBuildError::InvalidLimits);
        }
        Ok(Self {
            max_changes,
            max_fields_per_change,
            max_text_bytes,
            max_value_bytes,
            max_review_validity_ms,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationBuildError {
    InvalidLimits,
    EmptyTarget,
    NoChanges,
    ChangeLimitExceeded {
        actual: u64,
        limit: u32,
    },
    FieldLimitExceeded {
        change: u32,
        actual: u64,
        limit: u32,
    },
    TextLimitExceeded {
        actual: u64,
        limit: u64,
    },
    ValueLimitExceeded {
        actual: u64,
        limit: u64,
    },
    ChangeEngineMismatch {
        change: u32,
    },
    EmptyFields {
        change: u32,
    },
    EmptyFieldName {
        change: u32,
        field: u32,
    },
    DuplicateField {
        change: u32,
        field: u32,
    },
    NonExecutableValue {
        change: u32,
        field: u32,
    },
    NullLocator {
        change: u32,
        field: u32,
    },
    InvalidExpiration {
        change: u32,
    },
    MixedClickHouseExecutionModels {
        change: u32,
    },
}

impl fmt::Display for MutationBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid mutation plan: {self:?}")
    }
}

impl Error for MutationBuildError {}

pub struct MutationPlan {
    mutation_id: MutationId,
    scope: OperationScope,
    revision: Revision,
    target: MutationTarget,
    changes: Vec<MutationChange>,
    text_bytes: u64,
    value_bytes: u64,
    max_review_validity_ms: u64,
}

impl MutationPlan {
    pub fn new(
        mutation_id: MutationId,
        scope: OperationScope,
        revision: Revision,
        target: MutationTarget,
        changes: Vec<MutationChange>,
        limits: MutationPlanLimits,
    ) -> Result<Self, MutationBuildError> {
        if target.has_empty_part() {
            return Err(MutationBuildError::EmptyTarget);
        }
        if changes.is_empty() {
            return Err(MutationBuildError::NoChanges);
        }
        if changes.len() as u64 > u64::from(limits.max_changes) {
            return Err(MutationBuildError::ChangeLimitExceeded {
                actual: changes.len() as u64,
                limit: limits.max_changes,
            });
        }
        let mut text_bytes = target.byte_len();
        let mut value_bytes = 0_u64;
        let mut clickhouse_insert = None;
        for (index, change) in changes.iter().enumerate() {
            let index = u32::try_from(index).unwrap_or(u32::MAX);
            validate_change_engine(change, target.engine(), index)?;
            if target.engine() == Engine::ClickHouse {
                let is_insert = matches!(change, MutationChange::InsertRow { .. });
                if clickhouse_insert.is_some_and(|current| current != is_insert) {
                    return Err(MutationBuildError::MixedClickHouseExecutionModels {
                        change: index,
                    });
                }
                clickhouse_insert = Some(is_insert);
            }
            validate_change(change, index, limits, &mut text_bytes, &mut value_bytes)?;
            if text_bytes > limits.max_text_bytes {
                return Err(MutationBuildError::TextLimitExceeded {
                    actual: text_bytes,
                    limit: limits.max_text_bytes,
                });
            }
            if value_bytes > limits.max_value_bytes {
                return Err(MutationBuildError::ValueLimitExceeded {
                    actual: value_bytes,
                    limit: limits.max_value_bytes,
                });
            }
        }
        Ok(Self {
            mutation_id,
            scope,
            revision,
            target,
            changes,
            text_bytes,
            value_bytes,
            max_review_validity_ms: limits.max_review_validity_ms,
        })
    }

    #[must_use]
    pub const fn mutation_id(&self) -> MutationId {
        self.mutation_id
    }

    #[must_use]
    pub const fn scope(&self) -> OperationScope {
        self.scope
    }

    #[must_use]
    pub const fn revision(&self) -> Revision {
        self.revision
    }

    #[must_use]
    pub const fn target(&self) -> &MutationTarget {
        &self.target
    }

    #[must_use]
    pub fn changes(&self) -> &[MutationChange] {
        &self.changes
    }

    #[must_use]
    pub fn execution_model(&self) -> MutationExecutionModel {
        match self.target.engine() {
            Engine::PostgreSql => MutationExecutionModel::PostgreSqlAtomicTransaction,
            Engine::ClickHouse
                if matches!(self.changes.first(), Some(MutationChange::InsertRow { .. })) =>
            {
                MutationExecutionModel::ClickHouseProgressiveInsertNonTransactional
            }
            Engine::ClickHouse => {
                MutationExecutionModel::ClickHouseAsynchronousMutationNonTransactional
            }
            Engine::Redis => MutationExecutionModel::RedisSequentialNoRollback,
        }
    }

    pub fn review(
        self,
        token_id: ReviewTokenId,
        issued_at_ms: u64,
        expires_at_ms: u64,
    ) -> Result<ReviewedMutationPlan, ReviewError> {
        let validity = expires_at_ms
            .checked_sub(issued_at_ms)
            .ok_or(ReviewError::InvalidExpiry)?;
        if validity == 0 || validity > self.max_review_validity_ms {
            return Err(ReviewError::InvalidExpiry);
        }
        Ok(ReviewedMutationPlan {
            plan: self,
            token_id,
            issued_at_ms,
            expires_at_ms,
        })
    }
}

impl fmt::Debug for MutationPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MutationPlan")
            .field("mutation_id", &self.mutation_id)
            .field("scope", &self.scope)
            .field("revision", &self.revision)
            .field("target", &self.target)
            .field("changes", &self.changes.len())
            .field("text_bytes", &self.text_bytes)
            .field("value_bytes", &self.value_bytes)
            .field("execution_model", &self.execution_model())
            .finish()
    }
}

pub struct ReviewedMutationPlan {
    plan: MutationPlan,
    token_id: ReviewTokenId,
    issued_at_ms: u64,
    expires_at_ms: u64,
}

impl ReviewedMutationPlan {
    pub fn authorize(
        self,
        now_ms: u64,
        expected_scope: OperationScope,
        expected_revision: Revision,
    ) -> Result<AuthorizedMutationPlan, ReviewError> {
        if now_ms < self.issued_at_ms {
            return Err(ReviewError::ClockBeforeIssue);
        }
        if now_ms >= self.expires_at_ms {
            return Err(ReviewError::Expired);
        }
        if self.plan.scope != expected_scope {
            return Err(ReviewError::ScopeMismatch);
        }
        if self.plan.revision != expected_revision {
            return Err(ReviewError::RevisionMismatch);
        }
        Ok(AuthorizedMutationPlan {
            plan: self.plan,
            token_id: self.token_id,
        })
    }
}

pub struct AuthorizedMutationPlan {
    plan: MutationPlan,
    token_id: ReviewTokenId,
}

impl AuthorizedMutationPlan {
    #[must_use]
    pub const fn plan(&self) -> &MutationPlan {
        &self.plan
    }

    #[must_use]
    pub const fn token_id(&self) -> ReviewTokenId {
        self.token_id
    }
}

/// Bounded owner for reviewed mutation authority crossing copyable boundaries.
///
/// Redemption removes the entry before validation. A stale scope, revision, or
/// clock therefore consumes authority and cannot be retried with new arguments.
pub struct MutationReviewRegistry {
    max_entries: usize,
    entries: BTreeMap<ReviewTokenId, ReviewedMutationPlan>,
}

impl MutationReviewRegistry {
    pub const MAX_ENTRIES: u32 = 4_096;

    pub const fn new(max_entries: u32) -> Result<Self, ReviewRegistryError> {
        if max_entries == 0 || max_entries > Self::MAX_ENTRIES {
            return Err(ReviewRegistryError::InvalidCapacity);
        }
        Ok(Self {
            max_entries: max_entries as usize,
            entries: BTreeMap::new(),
        })
    }

    pub fn insert(
        &mut self,
        reviewed: ReviewedMutationPlan,
        now_ms: u64,
    ) -> Result<(), ReviewRegistryError> {
        if now_ms < reviewed.issued_at_ms {
            return Err(ReviewRegistryError::Review(ReviewError::ClockBeforeIssue));
        }
        if now_ms >= reviewed.expires_at_ms {
            return Err(ReviewRegistryError::Review(ReviewError::Expired));
        }
        self.purge_expired(now_ms);
        if self.entries.contains_key(&reviewed.token_id) {
            return Err(ReviewRegistryError::DuplicateToken);
        }
        if self.entries.len() >= self.max_entries {
            return Err(ReviewRegistryError::CapacityExceeded);
        }
        self.entries.insert(reviewed.token_id, reviewed);
        Ok(())
    }

    pub fn authorize(
        &mut self,
        token_id: ReviewTokenId,
        now_ms: u64,
        expected_scope: OperationScope,
        expected_revision: Revision,
    ) -> Result<AuthorizedMutationPlan, ReviewRegistryError> {
        let reviewed = self
            .entries
            .remove(&token_id)
            .ok_or(ReviewRegistryError::TokenNotFound)?;
        reviewed
            .authorize(now_ms, expected_scope, expected_revision)
            .map_err(ReviewRegistryError::Review)
    }

    pub fn revoke(&mut self, token_id: ReviewTokenId) -> bool {
        self.entries.remove(&token_id).is_some()
    }

    pub fn purge_expired(&mut self, now_ms: u64) -> usize {
        let before = self.entries.len();
        self.entries
            .retain(|_, reviewed| now_ms < reviewed.expires_at_ms);
        before - self.entries.len()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl fmt::Debug for MutationReviewRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MutationReviewRegistry")
            .field("entries", &self.entries.len())
            .field("max_entries", &self.max_entries)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewRegistryError {
    InvalidCapacity,
    CapacityExceeded,
    DuplicateToken,
    TokenNotFound,
    Review(ReviewError),
}

impl fmt::Display for ReviewRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidCapacity => "mutation review registry capacity must be nonzero",
            Self::CapacityExceeded => "mutation review registry capacity exceeded",
            Self::DuplicateToken => "mutation review token already exists",
            Self::TokenNotFound => "mutation review token is unavailable",
            Self::Review(_) => "mutation review authorization failed",
        })
    }
}

impl Error for ReviewRegistryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Review(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewError {
    InvalidExpiry,
    ClockBeforeIssue,
    Expired,
    ScopeMismatch,
    RevisionMismatch,
}

impl fmt::Display for ReviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidExpiry => "review expiry is outside the plan policy",
            Self::ClockBeforeIssue => "review clock precedes token issue",
            Self::Expired => "mutation review expired",
            Self::ScopeMismatch => "mutation review scope changed",
            Self::RevisionMismatch => "mutation plan revision changed",
        })
    }
}

impl Error for ReviewError {}

fn validate_change(
    change: &MutationChange,
    index: u32,
    limits: MutationPlanLimits,
    text_bytes: &mut u64,
    value_bytes: &mut u64,
) -> Result<(), MutationBuildError> {
    match change {
        MutationChange::InsertRow { values } => {
            validate_fields(values, false, index, limits, text_bytes, value_bytes)
        }
        MutationChange::UpdateRow {
            locator,
            assignments,
        } => {
            let total = locator.len().saturating_add(assignments.len()) as u64;
            if total > u64::from(limits.max_fields_per_change) {
                return Err(MutationBuildError::FieldLimitExceeded {
                    change: index,
                    actual: total,
                    limit: limits.max_fields_per_change,
                });
            }
            validate_fields(locator, true, index, limits, text_bytes, value_bytes)?;
            validate_fields(assignments, false, index, limits, text_bytes, value_bytes)
        }
        MutationChange::DeleteRow { locator } => {
            validate_fields(locator, true, index, limits, text_bytes, value_bytes)
        }
        MutationChange::RedisSetString { value, expiration } => {
            validate_expiration(*expiration, index)?;
            *value_bytes = value_bytes
                .checked_add(portable_len(value.len()))
                .unwrap_or(u64::MAX);
            Ok(())
        }
        MutationChange::RedisSetExpiration(RedisExpiration::Preserve) => {
            Err(MutationBuildError::InvalidExpiration { change: index })
        }
        MutationChange::RedisSetExpiration(expiration) => validate_expiration(*expiration, index),
        MutationChange::RedisDeleteKey => Ok(()),
    }
}

fn validate_change_engine(
    change: &MutationChange,
    engine: Engine,
    index: u32,
) -> Result<(), MutationBuildError> {
    let valid = match change {
        MutationChange::InsertRow { .. }
        | MutationChange::UpdateRow { .. }
        | MutationChange::DeleteRow { .. } => {
            matches!(engine, Engine::PostgreSql | Engine::ClickHouse)
        }
        MutationChange::RedisSetString { .. }
        | MutationChange::RedisDeleteKey
        | MutationChange::RedisSetExpiration(_) => engine == Engine::Redis,
    };
    valid
        .then_some(())
        .ok_or(MutationBuildError::ChangeEngineMismatch { change: index })
}

fn validate_fields(
    fields: &[FieldValue],
    locator: bool,
    change: u32,
    limits: MutationPlanLimits,
    text_bytes: &mut u64,
    value_bytes: &mut u64,
) -> Result<(), MutationBuildError> {
    if fields.is_empty() {
        return Err(MutationBuildError::EmptyFields { change });
    }
    if fields.len() as u64 > u64::from(limits.max_fields_per_change) {
        return Err(MutationBuildError::FieldLimitExceeded {
            change,
            actual: fields.len() as u64,
            limit: limits.max_fields_per_change,
        });
    }
    let mut names = BTreeSet::new();
    for (field_index, field) in fields.iter().enumerate() {
        let field_index = u32::try_from(field_index).unwrap_or(u32::MAX);
        if field.field.is_empty() {
            return Err(MutationBuildError::EmptyFieldName {
                change,
                field: field_index,
            });
        }
        if !names.insert(field.field.as_str()) {
            return Err(MutationBuildError::DuplicateField {
                change,
                field: field_index,
            });
        }
        if field.value.is_truncated()
            || matches!(
                field.value.kind(),
                ValueKind::Invalid | ValueKind::Unknown | ValueKind::Structured
            )
        {
            return Err(MutationBuildError::NonExecutableValue {
                change,
                field: field_index,
            });
        }
        if locator && field.value.kind() == ValueKind::Null {
            return Err(MutationBuildError::NullLocator {
                change,
                field: field_index,
            });
        }
        *text_bytes = text_bytes
            .checked_add(portable_len(field.field.len()))
            .unwrap_or(u64::MAX);
        *value_bytes = value_bytes
            .checked_add(field.value.encoded_byte_len())
            .unwrap_or(u64::MAX);
    }
    Ok(())
}

fn validate_expiration(expiration: RedisExpiration, change: u32) -> Result<(), MutationBuildError> {
    if matches!(
        expiration,
        RedisExpiration::ExpireAfterMillis(0 | 9_223_372_036_854_775_808..)
    ) {
        Err(MutationBuildError::InvalidExpiration { change })
    } else {
        Ok(())
    }
}

fn portable_len(length: usize) -> u64 {
    u64::try_from(length).unwrap_or(u64::MAX)
}
