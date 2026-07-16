use std::{error::Error, fmt};

use crate::{
    Engine, ProfileGroupName, ProfileId, ProfileName, ProfileSafetyMode, PropertyValueSource,
    Revision,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProfileListCursor {
    favorite: bool,
    saved_order: u32,
    id: ProfileId,
}

impl ProfileListCursor {
    #[must_use]
    pub const fn new(favorite: bool, saved_order: u32, id: ProfileId) -> Self {
        Self {
            favorite,
            saved_order,
            id,
        }
    }

    #[must_use]
    pub const fn favorite(self) -> bool {
        self.favorite
    }

    #[must_use]
    pub const fn saved_order(self) -> u32 {
        self.saved_order
    }

    #[must_use]
    pub const fn id(self) -> ProfileId {
        self.id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileListRequest {
    after: Option<ProfileListCursor>,
    limit: u16,
}

impl ProfileListRequest {
    pub const MAX_ITEMS: u16 = 100;

    pub const fn new(
        after: Option<ProfileListCursor>,
        limit: u16,
    ) -> Result<Self, ProfileListError> {
        if limit == 0 || limit > Self::MAX_ITEMS {
            return Err(ProfileListError::InvalidLimit {
                actual: limit,
                maximum: Self::MAX_ITEMS,
            });
        }
        Ok(Self { after, limit })
    }

    #[must_use]
    pub const fn after(self) -> Option<ProfileListCursor> {
        self.after
    }

    #[must_use]
    pub const fn limit(self) -> u16 {
        self.limit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProfileSourceFacts {
    host: PropertyValueSource,
    port: PropertyValueSource,
    has_secret_sources: bool,
    has_dangerous_plaintext: bool,
}

impl ProfileSourceFacts {
    #[must_use]
    pub const fn new(
        host: PropertyValueSource,
        port: PropertyValueSource,
        has_secret_sources: bool,
        has_dangerous_plaintext: bool,
    ) -> Self {
        Self {
            host,
            port,
            has_secret_sources,
            has_dangerous_plaintext,
        }
    }

    #[must_use]
    pub const fn host(self) -> PropertyValueSource {
        self.host
    }

    #[must_use]
    pub const fn port(self) -> PropertyValueSource {
        self.port
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
    pub const fn sources(&self) -> ProfileSourceFacts {
        self.sources
    }
    #[must_use]
    pub const fn cursor(&self) -> ProfileListCursor {
        ProfileListCursor::new(self.favorite, self.saved_order, self.id)
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
        request: ProfileListRequest,
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
        let next = has_more.then(|| items.last().expect("checked nonempty").cursor());
        Ok(Self { items, next })
    }

    #[must_use]
    pub fn items(&self) -> &[ProfileListItem] {
        &self.items
    }
    #[must_use]
    pub const fn next(&self) -> Option<ProfileListCursor> {
        self.next
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileListError {
    InvalidLimit { actual: u16, maximum: u16 },
    TooManyItems { actual: usize, maximum: u16 },
    EmptyContinuation,
}

impl fmt::Display for ProfileListError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("profile list contract is invalid")
    }
}

impl Error for ProfileListError {}
