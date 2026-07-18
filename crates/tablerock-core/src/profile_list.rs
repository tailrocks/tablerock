use std::{error::Error, fmt};

use caseless::Caseless;
use unicode_normalization::UnicodeNormalization;

use crate::{
    BoundedText, Engine, EnvironmentTag, ProfileGroupName, ProfileId, ProfileName, ProfileProperty,
    ProfilePropertyBinding, ProfilePropertyError, ProfileSafetyMode, PropertyValueSource, Revision,
};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ProfileSearchTerm {
    normalized: String,
}

impl ProfileSearchTerm {
    pub const MAX_INPUT_BYTES: u64 = 128;
    pub const MAX_NORMALIZED_BYTES: usize = 1_024;

    pub fn new(value: BoundedText) -> Result<Self, ProfileListError> {
        let actual = value.len() as u64;
        if actual == 0 || actual > Self::MAX_INPUT_BYTES {
            return Err(ProfileListError::InvalidSearchLength {
                actual,
                maximum: Self::MAX_INPUT_BYTES,
            });
        }
        if value.as_str().chars().any(char::is_control) {
            return Err(ProfileListError::InvalidSearchCharacter);
        }
        let normalized = normalize_search(value.as_str().trim());
        if normalized.is_empty() {
            return Err(ProfileListError::InvalidSearchCharacter);
        }
        if normalized.len() > Self::MAX_NORMALIZED_BYTES {
            return Err(ProfileListError::NormalizedSearchTooLong {
                actual: normalized.len(),
                maximum: Self::MAX_NORMALIZED_BYTES,
            });
        }
        Ok(Self { normalized })
    }

    #[must_use]
    pub fn matches(&self, candidate: &str) -> bool {
        normalize_search(candidate).contains(&self.normalized)
    }
}

impl fmt::Debug for ProfileSearchTerm {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProfileSearchTerm")
            .field("normalized_byte_len", &self.normalized.len())
            .finish()
    }
}

fn normalize_search(value: &str) -> String {
    value.chars().nfkc().default_case_fold().nfkc().collect()
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ProfileListFilter {
    engine: Option<Engine>,
    favorite: Option<bool>,
    group: Option<ProfileGroupName>,
    tag: Option<crate::ProfileTag>,
    environment: Option<EnvironmentTag>,
    search: Option<ProfileSearchTerm>,
}

impl ProfileListFilter {
    #[must_use]
    pub const fn new(engine: Option<Engine>, favorite: Option<bool>) -> Self {
        Self {
            engine,
            favorite,
            group: None,
            tag: None,
            environment: None,
            search: None,
        }
    }

    #[must_use]
    pub fn with_group(mut self, group: Option<ProfileGroupName>) -> Self {
        self.group = group;
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: Option<crate::ProfileTag>) -> Self {
        self.tag = tag;
        self
    }

    #[must_use]
    pub fn with_environment(mut self, environment: Option<EnvironmentTag>) -> Self {
        self.environment = environment;
        self
    }

    #[must_use]
    pub fn with_search(mut self, search: Option<ProfileSearchTerm>) -> Self {
        self.search = search;
        self
    }

    #[must_use]
    pub const fn engine(&self) -> Option<Engine> {
        self.engine
    }

    #[must_use]
    pub const fn favorite(&self) -> Option<bool> {
        self.favorite
    }

    #[must_use]
    pub const fn group(&self) -> Option<&ProfileGroupName> {
        self.group.as_ref()
    }

    #[must_use]
    pub const fn tag(&self) -> Option<&crate::ProfileTag> {
        self.tag.as_ref()
    }

    #[must_use]
    pub const fn environment(&self) -> Option<&EnvironmentTag> {
        self.environment.as_ref()
    }

    #[must_use]
    pub const fn search(&self) -> Option<&ProfileSearchTerm> {
        self.search.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProfileListCursor {
    filter: ProfileListFilter,
    favorite: bool,
    saved_order: u32,
    id: ProfileId,
}

impl ProfileListCursor {
    #[must_use]
    pub fn new(favorite: bool, saved_order: u32, id: ProfileId) -> Self {
        Self::in_filter(
            ProfileListFilter::new(None, None),
            favorite,
            saved_order,
            id,
        )
    }

    #[must_use]
    pub const fn in_filter(
        filter: ProfileListFilter,
        favorite: bool,
        saved_order: u32,
        id: ProfileId,
    ) -> Self {
        Self {
            filter,
            favorite,
            saved_order,
            id,
        }
    }

    #[must_use]
    pub const fn filter(&self) -> &ProfileListFilter {
        &self.filter
    }

    #[must_use]
    pub const fn favorite(&self) -> bool {
        self.favorite
    }

    #[must_use]
    pub const fn saved_order(&self) -> u32 {
        self.saved_order
    }

    #[must_use]
    pub const fn id(&self) -> ProfileId {
        self.id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileListRequest {
    filter: ProfileListFilter,
    after: Option<ProfileListCursor>,
    limit: u16,
}

impl ProfileListRequest {
    pub const MAX_ITEMS: u16 = 100;
    pub const MAX_SEARCH_CANDIDATES: usize = 10_000;

    pub fn new(
        filter: ProfileListFilter,
        after: Option<ProfileListCursor>,
        limit: u16,
    ) -> Result<Self, ProfileListError> {
        if limit == 0 || limit > Self::MAX_ITEMS {
            return Err(ProfileListError::InvalidLimit {
                actual: limit,
                maximum: Self::MAX_ITEMS,
            });
        }
        if let Some(cursor) = after.as_ref()
            && cursor.filter != filter
        {
            return Err(ProfileListError::CursorFilterMismatch);
        }
        Ok(Self {
            filter,
            after,
            limit,
        })
    }

    #[must_use]
    pub const fn filter(&self) -> &ProfileListFilter {
        &self.filter
    }

    #[must_use]
    pub const fn after(&self) -> Option<&ProfileListCursor> {
        self.after.as_ref()
    }

    #[must_use]
    pub const fn limit(&self) -> u16 {
        self.limit
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProfileEndpointPart {
    Literal(BoundedText),
    SecretSource,
}

impl ProfileEndpointPart {
    pub fn literal_host(value: BoundedText) -> Result<Self, ProfilePropertyError> {
        ProfilePropertyBinding::literal(ProfileProperty::Host, value.clone())?;
        Ok(Self::Literal(value))
    }

    pub fn literal_port(value: BoundedText) -> Result<Self, ProfilePropertyError> {
        ProfilePropertyBinding::literal(ProfileProperty::Port, value.clone())?;
        Ok(Self::Literal(value))
    }

    pub fn literal_context(value: BoundedText) -> Result<Self, ProfilePropertyError> {
        ProfilePropertyBinding::literal(ProfileProperty::DefaultContext, value.clone())?;
        Ok(Self::Literal(value))
    }

    #[must_use]
    pub const fn secret_source() -> Self {
        Self::SecretSource
    }

    #[must_use]
    pub const fn source(&self) -> PropertyValueSource {
        match self {
            Self::Literal(_) => PropertyValueSource::Literal,
            Self::SecretSource => PropertyValueSource::SecretSource,
        }
    }

    #[must_use]
    pub fn literal_value(&self) -> Option<&str> {
        match self {
            Self::Literal(value) => Some(value.as_str()),
            Self::SecretSource => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProfileEndpointSummary {
    host: ProfileEndpointPart,
    port: ProfileEndpointPart,
    context: Option<ProfileEndpointPart>,
}

impl ProfileEndpointSummary {
    #[must_use]
    pub const fn new(
        host: ProfileEndpointPart,
        port: ProfileEndpointPart,
        context: Option<ProfileEndpointPart>,
    ) -> Self {
        Self {
            host,
            port,
            context,
        }
    }

    #[must_use]
    pub const fn host(&self) -> &ProfileEndpointPart {
        &self.host
    }

    #[must_use]
    pub const fn port(&self) -> &ProfileEndpointPart {
        &self.port
    }

    #[must_use]
    pub const fn context(&self) -> Option<&ProfileEndpointPart> {
        self.context.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProfileSourceFacts {
    has_secret_sources: bool,
    has_dangerous_plaintext: bool,
}

impl ProfileSourceFacts {
    #[must_use]
    pub const fn new(has_secret_sources: bool, has_dangerous_plaintext: bool) -> Self {
        Self {
            has_secret_sources,
            has_dangerous_plaintext,
        }
    }

    #[must_use]
    pub const fn has_secret_sources(self) -> bool {
        self.has_secret_sources
    }

    #[must_use]
    pub const fn has_dangerous_plaintext(self) -> bool {
        self.has_dangerous_plaintext
    }
}

#[derive(PartialEq, Eq)]
pub struct ProfileListItem {
    id: ProfileId,
    revision: Revision,
    engine: Engine,
    name: ProfileName,
    group: Option<ProfileGroupName>,
    favorite: bool,
    saved_order: u32,
    safety_mode: ProfileSafetyMode,
    environment: Option<EnvironmentTag>,
    endpoint: ProfileEndpointSummary,
    sources: ProfileSourceFacts,
}

impl ProfileListItem {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        id: ProfileId,
        revision: Revision,
        engine: Engine,
        name: ProfileName,
        group: Option<ProfileGroupName>,
        favorite: bool,
        saved_order: u32,
        safety_mode: ProfileSafetyMode,
        environment: Option<EnvironmentTag>,
        endpoint: ProfileEndpointSummary,
        sources: ProfileSourceFacts,
    ) -> Self {
        Self {
            id,
            revision,
            engine,
            name,
            group,
            favorite,
            saved_order,
            safety_mode,
            environment,
            endpoint,
            sources,
        }
    }

    #[must_use]
    pub const fn id(&self) -> ProfileId {
        self.id
    }
    #[must_use]
    pub const fn revision(&self) -> Revision {
        self.revision
    }
    #[must_use]
    pub const fn engine(&self) -> Engine {
        self.engine
    }
    #[must_use]
    pub const fn name(&self) -> &ProfileName {
        &self.name
    }
    #[must_use]
    pub const fn group(&self) -> Option<&ProfileGroupName> {
        self.group.as_ref()
    }
    #[must_use]
    pub const fn favorite(&self) -> bool {
        self.favorite
    }
    #[must_use]
    pub const fn saved_order(&self) -> u32 {
        self.saved_order
    }
    #[must_use]
    pub const fn safety_mode(&self) -> ProfileSafetyMode {
        self.safety_mode
    }
    #[must_use]
    pub const fn environment(&self) -> Option<&EnvironmentTag> {
        self.environment.as_ref()
    }
    #[must_use]
    pub const fn endpoint(&self) -> &ProfileEndpointSummary {
        &self.endpoint
    }
    #[must_use]
    pub const fn sources(&self) -> ProfileSourceFacts {
        self.sources
    }
    #[must_use]
    pub const fn cursor(&self, filter: ProfileListFilter) -> ProfileListCursor {
        ProfileListCursor::in_filter(filter, self.favorite, self.saved_order, self.id)
    }
}

impl fmt::Debug for ProfileListItem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProfileListItem")
            .field("id", &self.id)
            .field("revision", &self.revision)
            .field("engine", &self.engine)
            .field("name", &self.name)
            .field("group", &self.group)
            .field("favorite", &self.favorite)
            .field("saved_order", &self.saved_order)
            .field("safety_mode", &self.safety_mode)
            .field("has_environment", &self.environment.is_some())
            .field("endpoint", &self.endpoint)
            .field("sources", &self.sources)
            .finish()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProfileListPage {
    items: Vec<ProfileListItem>,
    next: Option<ProfileListCursor>,
}

impl ProfileListPage {
    pub fn new(
        request: &ProfileListRequest,
        items: Vec<ProfileListItem>,
        has_more: bool,
    ) -> Result<Self, ProfileListError> {
        if items.len() > usize::from(request.limit()) {
            return Err(ProfileListError::TooManyItems {
                actual: items.len(),
                maximum: request.limit(),
            });
        }
        if has_more && items.is_empty() {
            return Err(ProfileListError::EmptyContinuation);
        }
        let next = has_more.then(|| {
            items
                .last()
                .expect("checked nonempty")
                .cursor(request.filter().clone())
        });
        Ok(Self { items, next })
    }

    #[must_use]
    pub fn items(&self) -> &[ProfileListItem] {
        &self.items
    }
    #[must_use]
    pub fn next(&self) -> Option<ProfileListCursor> {
        self.next.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileListError {
    InvalidLimit { actual: u16, maximum: u16 },
    TooManyItems { actual: usize, maximum: u16 },
    EmptyContinuation,
    CursorFilterMismatch,
    InvalidSearchLength { actual: u64, maximum: u64 },
    InvalidSearchCharacter,
    NormalizedSearchTooLong { actual: usize, maximum: usize },
}

impl fmt::Display for ProfileListError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("profile list contract is invalid")
    }
}

impl Error for ProfileListError {}
