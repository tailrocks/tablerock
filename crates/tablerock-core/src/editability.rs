//! Result editability proof — base table + stable identity + safety mode.
//!
//! Joins, aggregates, key-less results, and ReadOnly profiles are never
//! editable. Unknown/truncated/invalid cells are never editable even when the
//! result is otherwise writable (cell-level gate lives with the value).

use crate::{ProfileSafetyMode, ValueKind};

/// Why a result (or cell) cannot be staged for mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditabilityReason {
    /// No single base relation attached to the result.
    NoBaseTable,
    /// Base table present but no primary/unique key columns proven.
    NoStableIdentity,
    /// Result shape is a join, aggregate, or other non-base projection.
    NonBaseResult,
    /// Profile policy forbids writes.
    ProfileReadOnly,
    /// Cell value kind cannot be written.
    ValueNotWritable,
}

impl EditabilityReason {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::NoBaseTable => "no base table",
            Self::NoStableIdentity => "no stable row identity",
            Self::NonBaseResult => "join, aggregate, or non-base result",
            Self::ProfileReadOnly => "profile is read only",
            Self::ValueNotWritable => "value not writable",
        }
    }
}

/// Proven identity columns for row location (primary or unique key).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StableIdentity {
    pub columns: Vec<String>,
}

impl StableIdentity {
    pub fn new(columns: Vec<String>) -> Option<Self> {
        let columns: Vec<_> = columns
            .into_iter()
            .map(|c| c.trim().to_owned())
            .filter(|c| !c.is_empty())
            .collect();
        if columns.is_empty() {
            None
        } else {
            Some(Self { columns })
        }
    }

    #[must_use]
    pub fn column_names(&self) -> &[String] {
        &self.columns
    }
}

/// Result-level editability facts (not presentation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditabilityFacts {
    /// Result may accept staged mutations; identity columns locate rows.
    Editable {
        schema: String,
        table: String,
        identity: StableIdentity,
    },
    /// Staging must not be offered; reason is operator-visible.
    ReadOnly { reason: EditabilityReason },
}

impl EditabilityFacts {
    /// Classify a result from base-table identity and profile safety.
    ///
    /// `non_base` is true for joins/aggregates/CTEs without a single base
    /// relation (caller proves shape; this module does not parse SQL).
    #[must_use]
    pub fn classify(
        safety: ProfileSafetyMode,
        non_base: bool,
        schema: Option<&str>,
        table: Option<&str>,
        identity_columns: &[String],
    ) -> Self {
        if matches!(safety, ProfileSafetyMode::ReadOnly) {
            return Self::ReadOnly {
                reason: EditabilityReason::ProfileReadOnly,
            };
        }
        if non_base {
            return Self::ReadOnly {
                reason: EditabilityReason::NonBaseResult,
            };
        }
        let (Some(schema), Some(table)) = (schema, table) else {
            return Self::ReadOnly {
                reason: EditabilityReason::NoBaseTable,
            };
        };
        let schema = schema.trim();
        let table = table.trim();
        if schema.is_empty() || table.is_empty() {
            return Self::ReadOnly {
                reason: EditabilityReason::NoBaseTable,
            };
        }
        let Some(identity) = StableIdentity::new(identity_columns.to_vec()) else {
            return Self::ReadOnly {
                reason: EditabilityReason::NoStableIdentity,
            };
        };
        Self::Editable {
            schema: schema.to_owned(),
            table: table.to_owned(),
            identity,
        }
    }

    #[must_use]
    pub const fn is_editable(&self) -> bool {
        matches!(self, Self::Editable { .. })
    }

    #[must_use]
    pub const fn reason(&self) -> Option<EditabilityReason> {
        match self {
            Self::Editable { .. } => None,
            Self::ReadOnly { reason } => Some(*reason),
        }
    }

    #[must_use]
    pub fn identity_columns(&self) -> &[String] {
        match self {
            Self::Editable { identity, .. } => identity.column_names(),
            Self::ReadOnly { .. } => &[],
        }
    }

    /// Cell-level gate: truncated/invalid/unknown never edit even if result is.
    #[must_use]
    pub fn cell_writable(kind: ValueKind, truncated: bool) -> Result<(), EditabilityReason> {
        if truncated {
            return Err(EditabilityReason::ValueNotWritable);
        }
        match kind {
            ValueKind::Invalid | ValueKind::Unknown => Err(EditabilityReason::ValueNotWritable),
            ValueKind::Null
            | ValueKind::Boolean
            | ValueKind::Signed
            | ValueKind::Unsigned
            | ValueKind::Float64
            | ValueKind::Decimal
            | ValueKind::Temporal
            | ValueKind::Text
            | ValueKind::Structured
            | ValueKind::Binary => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_profile_blocks_even_with_identity() {
        let facts = EditabilityFacts::classify(
            ProfileSafetyMode::ReadOnly,
            false,
            Some("public"),
            Some("users"),
            &["id".into()],
        );
        assert!(!facts.is_editable());
        assert_eq!(facts.reason(), Some(EditabilityReason::ProfileReadOnly));
    }

    #[test]
    fn confirm_writes_with_pk_is_editable() {
        let facts = EditabilityFacts::classify(
            ProfileSafetyMode::ConfirmWrites,
            false,
            Some("public"),
            Some("users"),
            &["id".into()],
        );
        assert!(facts.is_editable());
        assert_eq!(facts.identity_columns(), &["id".to_owned()]);
    }

    #[test]
    fn join_and_keyless_are_read_only() {
        assert_eq!(
            EditabilityFacts::classify(
                ProfileSafetyMode::ConfirmWrites,
                true,
                Some("public"),
                Some("users"),
                &["id".into()],
            )
            .reason(),
            Some(EditabilityReason::NonBaseResult)
        );
        assert_eq!(
            EditabilityFacts::classify(
                ProfileSafetyMode::ConfirmWrites,
                false,
                Some("public"),
                Some("users"),
                &[],
            )
            .reason(),
            Some(EditabilityReason::NoStableIdentity)
        );
        assert_eq!(
            EditabilityFacts::classify(
                ProfileSafetyMode::ConfirmWrites,
                false,
                None,
                None,
                &["id".into()],
            )
            .reason(),
            Some(EditabilityReason::NoBaseTable)
        );
    }

    #[test]
    fn truncated_and_unknown_cells_not_writable() {
        assert!(EditabilityFacts::cell_writable(ValueKind::Text, true).is_err());
        assert!(EditabilityFacts::cell_writable(ValueKind::Unknown, false).is_err());
        assert!(EditabilityFacts::cell_writable(ValueKind::Invalid, false).is_err());
        assert!(EditabilityFacts::cell_writable(ValueKind::Signed, false).is_ok());
        assert!(EditabilityFacts::cell_writable(ValueKind::Null, false).is_ok());
    }
}
