use std::{collections::BTreeMap, error::Error, fmt};

use crate::{Engine, PageIdentity, ResultId, ResultPage, Revision, RevisionRelation};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResultStoreLimits {
    max_results: u32,
    max_pages: u32,
    max_resident_buffer_bytes: u64,
}

impl ResultStoreLimits {
    pub const fn new(
        max_results: u32,
        max_pages: u32,
        max_resident_buffer_bytes: u64,
    ) -> Result<Self, ResultStoreError> {
        if max_results == 0 || max_pages == 0 || max_resident_buffer_bytes == 0 {
            return Err(ResultStoreError::InvalidLimits);
        }
        Ok(Self {
            max_results,
            max_pages,
            max_resident_buffer_bytes,
        })
    }

    #[must_use]
    pub const fn max_results(self) -> u32 {
        self.max_results
    }

    #[must_use]
    pub const fn max_pages(self) -> u32 {
        self.max_pages
    }

    #[must_use]
    pub const fn max_resident_buffer_bytes(self) -> u64 {
        self.max_resident_buffer_bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageKey {
    result_id: ResultId,
    revision: Revision,
    start_row: u64,
}

impl PageKey {
    #[must_use]
    pub const fn new(result_id: ResultId, revision: Revision, start_row: u64) -> Self {
        Self {
            result_id,
            revision,
            start_row,
        }
    }

    #[must_use]
    pub const fn result_id(self) -> ResultId {
        self.result_id
    }

    #[must_use]
    pub const fn revision(self) -> Revision {
        self.revision
    }

    #[must_use]
    pub const fn start_row(self) -> u64 {
        self.start_row
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenResultOutcome {
    Opened,
    AlreadyOpen,
    Replaced { evicted: Vec<PageKey> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionOutcome {
    admitted: PageKey,
    evicted: Vec<PageKey>,
}

impl AdmissionOutcome {
    #[must_use]
    pub const fn admitted(&self) -> PageKey {
        self.admitted
    }

    #[must_use]
    pub fn evicted(&self) -> &[PageKey] {
        &self.evicted
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultStoreError {
    InvalidLimits,
    ResultSlotsFull,
    ResultNotOpen,
    StaleRevision,
    FutureRevision,
    EngineMismatch,
    DuplicatePage,
    OverlappingPage,
    PageTooLarge { actual: u64, limit: u64 },
    PinnedCapacity,
}

impl fmt::Display for ResultStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLimits => "result-store limits must be finite and nonzero",
            Self::ResultSlotsFull => "result store has no free result slot",
            Self::ResultNotOpen => "result is not open",
            Self::StaleRevision => "result page revision is stale",
            Self::FutureRevision => "result page revision was not opened",
            Self::EngineMismatch => "result page engine does not match the open result",
            Self::DuplicatePage => "result page is already resident",
            Self::OverlappingPage => "result page overlaps a resident page",
            Self::PageTooLarge { .. } => "result page exceeds the resident byte budget",
            Self::PinnedCapacity => "pinned result pages prevent bounded admission",
        })
    }
}

impl Error for ResultStoreError {}

struct StoredPage {
    page: ResultPage,
    resident_bytes: u64,
    last_access: u64,
    pinned: bool,
}

struct ResultSlot {
    revision: Revision,
    engine: Engine,
    pages: BTreeMap<u64, StoredPage>,
}

pub struct ResultStore {
    limits: ResultStoreLimits,
    results: BTreeMap<ResultId, ResultSlot>,
    page_count: u32,
    resident_buffer_bytes: u64,
    access_clock: u64,
}

impl ResultStore {
    #[must_use]
    pub fn new(limits: ResultStoreLimits) -> Self {
        Self {
            limits,
            results: BTreeMap::new(),
            page_count: 0,
            resident_buffer_bytes: 0,
            access_clock: 0,
        }
    }

    pub fn open_result(
        &mut self,
        identity: PageIdentity,
    ) -> Result<OpenResultOutcome, ResultStoreError> {
        if let Some(slot) = self.results.get(&identity.result_id()) {
            if identity.engine() != slot.engine {
                return Err(ResultStoreError::EngineMismatch);
            }
            match identity.revision().relation_to(slot.revision) {
                RevisionRelation::Stale => return Err(ResultStoreError::StaleRevision),
                RevisionRelation::Current => return Ok(OpenResultOutcome::AlreadyOpen),
                RevisionRelation::Future => {}
            }
        } else {
            if self.results.len() >= self.limits.max_results as usize {
                return Err(ResultStoreError::ResultSlotsFull);
            }
            self.results.insert(
                identity.result_id(),
                ResultSlot {
                    revision: identity.revision(),
                    engine: identity.engine(),
                    pages: BTreeMap::new(),
                },
            );
            return Ok(OpenResultOutcome::Opened);
        }

        let slot = self
            .results
            .get_mut(&identity.result_id())
            .expect("checked above");
        let evicted = slot
            .pages
            .keys()
            .map(|start| PageKey::new(identity.result_id(), slot.revision, *start))
            .collect::<Vec<_>>();
        for page in slot.pages.values() {
            self.resident_buffer_bytes = self
                .resident_buffer_bytes
                .saturating_sub(page.resident_bytes);
        }
        self.page_count = self
            .page_count
            .saturating_sub(u32::try_from(slot.pages.len()).unwrap_or(u32::MAX));
        slot.pages.clear();
        slot.revision = identity.revision();
        slot.engine = identity.engine();
        Ok(OpenResultOutcome::Replaced { evicted })
    }

    pub fn close_result(&mut self, result_id: ResultId) -> Vec<PageKey> {
        let Some(slot) = self.results.remove(&result_id) else {
            return Vec::new();
        };
        let mut evicted = Vec::with_capacity(slot.pages.len());
        for (start, page) in slot.pages {
            self.page_count = self.page_count.saturating_sub(1);
            self.resident_buffer_bytes = self
                .resident_buffer_bytes
                .saturating_sub(page.resident_bytes);
            evicted.push(PageKey::new(result_id, slot.revision, start));
        }
        evicted
    }

    pub fn admit(&mut self, page: ResultPage) -> Result<AdmissionOutcome, ResultStoreError> {
        let envelope = page.envelope();
        let key = PageKey::new(
            envelope.result_id(),
            envelope.revision(),
            envelope.start_row(),
        );
        let Some(slot) = self.results.get(&key.result_id) else {
            return Err(ResultStoreError::ResultNotOpen);
        };
        match key.revision.relation_to(slot.revision) {
            RevisionRelation::Stale => return Err(ResultStoreError::StaleRevision),
            RevisionRelation::Future => return Err(ResultStoreError::FutureRevision),
            RevisionRelation::Current => {}
        }
        if envelope.engine() != slot.engine {
            return Err(ResultStoreError::EngineMismatch);
        }
        if slot.pages.contains_key(&key.start_row) {
            return Err(ResultStoreError::DuplicatePage);
        }
        let end = key
            .start_row
            .saturating_add(u64::from(envelope.row_count()));
        if slot.pages.iter().any(|(start, resident)| {
            let resident_end =
                start.saturating_add(u64::from(resident.page.envelope().row_count()));
            key.start_row < resident_end && *start < end
        }) {
            return Err(ResultStoreError::OverlappingPage);
        }
        let resident_bytes = page.resident_buffer_bytes();
        if resident_bytes > self.limits.max_resident_buffer_bytes {
            return Err(ResultStoreError::PageTooLarge {
                actual: resident_bytes,
                limit: self.limits.max_resident_buffer_bytes,
            });
        }

        let required_pages = u64::from(self.page_count) + 1;
        let required_bytes = u128::from(self.resident_buffer_bytes) + u128::from(resident_bytes);
        let mut candidates = self.eviction_candidates();
        candidates.sort_by_key(|candidate| (candidate.0, candidate.1));
        let mut selected = Vec::new();
        let mut freed_pages = 0_u64;
        let mut freed_bytes = 0_u128;
        for (_, candidate) in candidates {
            if required_pages.saturating_sub(freed_pages) <= u64::from(self.limits.max_pages)
                && required_bytes.saturating_sub(freed_bytes)
                    <= u128::from(self.limits.max_resident_buffer_bytes)
            {
                break;
            }
            let candidate_page = &self.results[&candidate.result_id].pages[&candidate.start_row];
            freed_pages = freed_pages.saturating_add(1);
            freed_bytes = freed_bytes.saturating_add(u128::from(candidate_page.resident_bytes));
            selected.push(candidate);
        }
        if required_pages.saturating_sub(freed_pages) > u64::from(self.limits.max_pages)
            || required_bytes.saturating_sub(freed_bytes)
                > u128::from(self.limits.max_resident_buffer_bytes)
        {
            return Err(ResultStoreError::PinnedCapacity);
        }

        for candidate in &selected {
            self.remove_page(*candidate);
        }
        let access = self.next_access();
        self.results
            .get_mut(&key.result_id)
            .expect("open result remains present")
            .pages
            .insert(
                key.start_row,
                StoredPage {
                    page,
                    resident_bytes,
                    last_access: access,
                    pinned: false,
                },
            );
        self.page_count = self
            .page_count
            .checked_add(1)
            .expect("admission proved page limit");
        self.resident_buffer_bytes = self
            .resident_buffer_bytes
            .checked_add(resident_bytes)
            .expect("admission proved byte limit");
        Ok(AdmissionOutcome {
            admitted: key,
            evicted: selected,
        })
    }

    pub fn get(&mut self, key: PageKey) -> Option<&ResultPage> {
        let access = self.next_access();
        let slot = self.results.get_mut(&key.result_id)?;
        if slot.revision != key.revision {
            return None;
        }
        let page = slot.pages.get_mut(&key.start_row)?;
        page.last_access = access;
        Some(&page.page)
    }

    /// Clones all resident pages for one current result in start-row order.
    /// Bounded by ResultStore limits; touching pages refreshes their LRU age.
    pub fn resident_pages(
        &mut self,
        result_id: ResultId,
        revision: Revision,
    ) -> Option<Vec<ResultPage>> {
        let starts = self.results.get(&result_id).and_then(|slot| {
            (slot.revision == revision).then(|| slot.pages.keys().copied().collect::<Vec<_>>())
        })?;
        starts
            .into_iter()
            .map(|start| self.get(PageKey::new(result_id, revision, start)).cloned())
            .collect()
    }

    pub fn set_pinned(&mut self, key: PageKey, pinned: bool) -> bool {
        let Some(slot) = self.results.get_mut(&key.result_id) else {
            return false;
        };
        if slot.revision != key.revision {
            return false;
        }
        let Some(page) = slot.pages.get_mut(&key.start_row) else {
            return false;
        };
        page.pinned = pinned;
        true
    }

    #[must_use]
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }

    #[must_use]
    pub const fn resident_buffer_bytes(&self) -> u64 {
        self.resident_buffer_bytes
    }

    fn eviction_candidates(&self) -> Vec<(u64, PageKey)> {
        self.results
            .iter()
            .flat_map(|(result_id, slot)| {
                slot.pages.iter().filter_map(move |(start, page)| {
                    (!page.pinned).then_some((
                        page.last_access,
                        PageKey::new(*result_id, slot.revision, *start),
                    ))
                })
            })
            .collect()
    }

    fn remove_page(&mut self, key: PageKey) {
        let page = self
            .results
            .get_mut(&key.result_id)
            .and_then(|slot| slot.pages.remove(&key.start_row))
            .expect("eviction candidate remains resident");
        self.page_count -= 1;
        self.resident_buffer_bytes -= page.resident_bytes;
    }

    fn next_access(&mut self) -> u64 {
        if self.access_clock == u64::MAX {
            let mut ordered = self
                .results
                .iter()
                .flat_map(|(result_id, slot)| {
                    slot.pages.iter().map(move |(start, page)| {
                        (
                            page.last_access,
                            PageKey::new(*result_id, slot.revision, *start),
                        )
                    })
                })
                .collect::<Vec<_>>();
            ordered.sort_by_key(|candidate| (candidate.0, candidate.1));
            for (index, (_, key)) in ordered.into_iter().enumerate() {
                if let Some(page) = self
                    .results
                    .get_mut(&key.result_id)
                    .and_then(|slot| slot.pages.get_mut(&key.start_row))
                {
                    page.last_access = u64::try_from(index).unwrap_or(u64::MAX - 1) + 1;
                }
            }
            self.access_clock = u64::from(self.page_count);
        }
        self.access_clock += 1;
        self.access_clock
    }
}

impl fmt::Debug for ResultStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResultStore")
            .field("results", &self.results.len())
            .field("pages", &self.page_count)
            .field("resident_buffer_bytes", &self.resident_buffer_bytes)
            .field("limits", &self.limits)
            .finish()
    }
}
