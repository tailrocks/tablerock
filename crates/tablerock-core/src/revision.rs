use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterOverflow;

impl fmt::Display for CounterOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("monotonic counter space exhausted")
    }
}

impl Error for CounterOverflow {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevisionRelation {
    Stale,
    Current,
    Future,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceRelation {
    StaleOrDuplicate,
    Next,
    Gap,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(u64);

impl Revision {
    pub const INITIAL: Self = Self(0);

    /// Restore a trusted wire/persistence value; aggregate validation remains mandatory.
    #[must_use]
    pub const fn from_wire_u64(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    pub const fn checked_next(self) -> Result<Self, CounterOverflow> {
        match self.0.checked_add(1) {
            Some(next) => Ok(Self(next)),
            None => Err(CounterOverflow),
        }
    }

    #[must_use]
    pub const fn relation_to(self, current: Self) -> RevisionRelation {
        if self.0 < current.0 {
            RevisionRelation::Stale
        } else if self.0 == current.0 {
            RevisionRelation::Current
        } else {
            RevisionRelation::Future
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventSequence(u64);

impl EventSequence {
    pub const INITIAL: Self = Self(0);

    /// Restore a trusted wire/persistence value; stream validation remains mandatory.
    #[must_use]
    pub const fn from_wire_u64(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    pub const fn checked_next(self) -> Result<Self, CounterOverflow> {
        match self.0.checked_add(1) {
            Some(next) => Ok(Self(next)),
            None => Err(CounterOverflow),
        }
    }

    #[must_use]
    pub const fn relation_to(self, last_seen: Self) -> SequenceRelation {
        if self.0 <= last_seen.0 {
            SequenceRelation::StaleOrDuplicate
        } else if self.0 == last_seen.0.saturating_add(1) {
            SequenceRelation::Next
        } else {
            SequenceRelation::Gap
        }
    }
}
