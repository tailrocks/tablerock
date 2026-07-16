use std::{error::Error, fmt};

use crate::{BoundedText, ProfileConnectionSnapshot, Revision};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ProfileGroupName(BoundedText);

impl ProfileGroupName {
    pub const MAX_BYTES: u64 = 128;

    pub fn new(value: BoundedText) -> Result<Self, ProfileAggregateError> {
        validate_label(&value, Self::MAX_BYTES, ProfileLabel::Group)?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for ProfileGroupName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProfileGroupName")
            .field("byte_len", &self.0.len())
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ProfileTag(BoundedText);

impl ProfileTag {
    pub const MAX_BYTES: u64 = 64;

    pub fn new(value: BoundedText) -> Result<Self, ProfileAggregateError> {
        validate_label(&value, Self::MAX_BYTES, ProfileLabel::Tag)?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for ProfileTag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProfileTag")
            .field("byte_len", &self.0.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileLabel {
    Group,
    Tag,
}

fn validate_label(
    value: &BoundedText,
    maximum: u64,
    label: ProfileLabel,
) -> Result<(), ProfileAggregateError> {
    let actual = value.len() as u64;
    if actual == 0 || actual > maximum {
        return Err(ProfileAggregateError::InvalidLabelLength {
            label,
            actual,
            maximum,
        });
    }
    if value.as_str().trim().is_empty() || value.as_str().chars().any(char::is_control) {
        return Err(ProfileAggregateError::InvalidLabelCharacter { label });
    }
    Ok(())
}

#[derive(PartialEq, Eq)]
pub struct ProfileOrganization {
    group: Option<ProfileGroupName>,
    tags: Vec<ProfileTag>,
    favorite: bool,
    order: u32,
}

impl ProfileOrganization {
    pub const MAX_TAGS: usize = 32;

    #[must_use]
    pub const fn empty() -> Self {
        Self {
            group: None,
            tags: Vec::new(),
            favorite: false,
            order: 0,
        }
    }

    pub fn new(
        group: Option<ProfileGroupName>,
        tags: Vec<ProfileTag>,
        favorite: bool,
        order: u32,
    ) -> Result<Self, ProfileAggregateError> {
        if tags.len() > Self::MAX_TAGS {
            return Err(ProfileAggregateError::TooManyTags {
                actual: tags.len(),
                maximum: Self::MAX_TAGS,
            });
        }
        for (duplicate_index, tag) in tags.iter().enumerate() {
            if let Some(first_index) = tags[..duplicate_index]
                .iter()
                .position(|candidate| candidate == tag)
            {
                return Err(ProfileAggregateError::DuplicateTag {
                    first_index,
                    duplicate_index,
                });
            }
        }
        Ok(Self {
            group,
            tags,
            favorite,
            order,
        })
    }

    #[must_use]
    pub const fn group(&self) -> Option<&ProfileGroupName> {
        self.group.as_ref()
    }
    #[must_use]
    pub fn tags(&self) -> &[ProfileTag] {
        &self.tags
    }
    #[must_use]
    pub const fn favorite(&self) -> bool {
        self.favorite
    }
    #[must_use]
    pub const fn order(&self) -> u32 {
        self.order
    }
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.group.is_none() && self.tags.is_empty() && !self.favorite && self.order == 0
    }
}

impl fmt::Debug for ProfileOrganization {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProfileOrganization")
            .field("has_group", &self.group.is_some())
            .field("tag_count", &self.tags.len())
            .field("favorite", &self.favorite)
            .field("order", &self.order)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReconnectPreference {
    Manual,
    BoundedAutomatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProfilePreferences {
    reconnect: ReconnectPreference,
    restore_last_context: bool,
    preferred_page_rows: u32,
}

impl ProfilePreferences {
    pub const MAX_PREFERRED_PAGE_ROWS: u32 = 500;

    pub fn new(
        reconnect: ReconnectPreference,
        restore_last_context: bool,
        preferred_page_rows: u32,
    ) -> Result<Self, ProfileAggregateError> {
        if preferred_page_rows == 0 || preferred_page_rows > Self::MAX_PREFERRED_PAGE_ROWS {
            return Err(ProfileAggregateError::InvalidPreferredPageRows {
                actual: preferred_page_rows,
                maximum: Self::MAX_PREFERRED_PAGE_ROWS,
            });
        }
        Ok(Self {
            reconnect,
            restore_last_context,
            preferred_page_rows,
        })
    }

    #[must_use]
    pub const fn reconnect(self) -> ReconnectPreference {
        self.reconnect
    }
    #[must_use]
    pub const fn restore_last_context(self) -> bool {
        self.restore_last_context
    }
    #[must_use]
    pub const fn preferred_page_rows(self) -> u32 {
        self.preferred_page_rows
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileDurability {
    Saved,
    Temporary,
}

#[derive(PartialEq, Eq)]
pub struct ProfileAggregate {
    schema_version: u16,
    connection: ProfileConnectionSnapshot,
    durability: ProfileDurability,
    organization: ProfileOrganization,
    preferences: ProfilePreferences,
}

impl ProfileAggregate {
    pub const SCHEMA_VERSION: u16 = 1;

    pub fn new(
        connection: ProfileConnectionSnapshot,
        durability: ProfileDurability,
        organization: ProfileOrganization,
        preferences: ProfilePreferences,
    ) -> Result<Self, ProfileAggregateError> {
        Self::from_wire(
            Self::SCHEMA_VERSION,
            connection,
            durability,
            organization,
            preferences,
        )
    }

    pub fn from_wire(
        schema_version: u16,
        connection: ProfileConnectionSnapshot,
        durability: ProfileDurability,
        organization: ProfileOrganization,
        preferences: ProfilePreferences,
    ) -> Result<Self, ProfileAggregateError> {
        if schema_version != Self::SCHEMA_VERSION {
            return Err(ProfileAggregateError::UnsupportedSchemaVersion {
                actual: schema_version,
                supported: Self::SCHEMA_VERSION,
            });
        }
        if durability == ProfileDurability::Temporary && !organization.is_empty() {
            return Err(ProfileAggregateError::TemporaryOrganizationForbidden);
        }
        Ok(Self {
            schema_version,
            connection,
            durability,
            organization,
            preferences,
        })
    }

    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }
    #[must_use]
    pub const fn connection(&self) -> &ProfileConnectionSnapshot {
        &self.connection
    }
    #[must_use]
    pub const fn durability(&self) -> ProfileDurability {
        self.durability
    }
    #[must_use]
    pub const fn organization(&self) -> &ProfileOrganization {
        &self.organization
    }
    #[must_use]
    pub const fn preferences(&self) -> ProfilePreferences {
        self.preferences
    }
    #[must_use]
    pub const fn persistable(&self) -> Option<PersistableProfile<'_>> {
        match self.durability {
            ProfileDurability::Saved => Some(PersistableProfile { profile: self }),
            ProfileDurability::Temporary => None,
        }
    }

    pub fn validate_replacement(
        &self,
        expected_revision: Revision,
        proposed: &Self,
    ) -> Result<(), ProfileUpdateError> {
        let current = self.connection.revision();
        if expected_revision != current {
            return Err(ProfileUpdateError::StaleRevision {
                expected: expected_revision,
                current,
            });
        }
        if self.connection.id() != proposed.connection.id() {
            return Err(ProfileUpdateError::IdentityMismatch);
        }
        if self.durability != proposed.durability {
            return Err(ProfileUpdateError::DurabilityChange);
        }
        let expected_next = current
            .checked_next()
            .map_err(|_| ProfileUpdateError::RevisionExhausted)?;
        let actual = proposed.connection.revision();
        if actual != expected_next {
            return Err(ProfileUpdateError::NonSequentialRevision {
                expected: expected_next,
                actual,
            });
        }
        Ok(())
    }
}

pub struct PersistableProfile<'a> {
    profile: &'a ProfileAggregate,
}

impl PersistableProfile<'_> {
    #[must_use]
    pub const fn profile(&self) -> &ProfileAggregate {
        self.profile
    }
}

impl fmt::Debug for PersistableProfile<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PersistableProfile")
            .field("id", &self.profile.connection.id())
            .field("revision", &self.profile.connection.revision())
            .finish()
    }
}

impl fmt::Debug for ProfileAggregate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProfileAggregate")
            .field("schema_version", &self.schema_version)
            .field("connection", &self.connection)
            .field("durability", &self.durability)
            .field("organization", &self.organization)
            .field("preferences", &self.preferences)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileAggregateError {
    InvalidLabelLength {
        label: ProfileLabel,
        actual: u64,
        maximum: u64,
    },
    InvalidLabelCharacter {
        label: ProfileLabel,
    },
    TooManyTags {
        actual: usize,
        maximum: usize,
    },
    DuplicateTag {
        first_index: usize,
        duplicate_index: usize,
    },
    InvalidPreferredPageRows {
        actual: u32,
        maximum: u32,
    },
    TemporaryOrganizationForbidden,
    UnsupportedSchemaVersion {
        actual: u16,
        supported: u16,
    },
}

impl fmt::Display for ProfileAggregateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLabelLength { .. } => "profile label length is invalid",
            Self::InvalidLabelCharacter { .. } => "profile label contains invalid characters",
            Self::TooManyTags { .. } => "profile tag count exceeds its bound",
            Self::DuplicateTag { .. } => "profile tags must be unique",
            Self::InvalidPreferredPageRows { .. } => "preferred page size is outside 1..=500",
            Self::TemporaryOrganizationForbidden => {
                "temporary profiles cannot carry saved organization metadata"
            }
            Self::UnsupportedSchemaVersion { .. } => {
                "profile aggregate schema version is unsupported"
            }
        })
    }
}

impl Error for ProfileAggregateError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileUpdateError {
    StaleRevision {
        expected: Revision,
        current: Revision,
    },
    IdentityMismatch,
    DurabilityChange,
    NonSequentialRevision {
        expected: Revision,
        actual: Revision,
    },
    RevisionExhausted,
}

impl fmt::Display for ProfileUpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::StaleRevision { .. } => "profile update revision is stale",
            Self::IdentityMismatch => "profile replacement identity differs",
            Self::DurabilityChange => "profile replacement cannot change durability",
            Self::NonSequentialRevision { .. } => "profile replacement revision is not next",
            Self::RevisionExhausted => "profile revision space is exhausted",
        })
    }
}

impl Error for ProfileUpdateError {}
