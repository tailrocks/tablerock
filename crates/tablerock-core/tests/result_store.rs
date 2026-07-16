use tablerock_core::{
    AdmissionOutcome, BoundedText, ByteLimit, ColumnMetadata, Engine, EngineType, IdParts,
    OpenResultOutcome, OwnedValue, PageDelivery, PageFacts, PageIdentity, PageLimits, PageWarnings,
    ResultId, ResultPage, ResultStore, ResultStoreError, ResultStoreLimits, Revision, RowTotal,
    Truncation,
};

fn result_id(low: u64) -> ResultId {
    ResultId::from_parts(IdParts::new(0, low).unwrap()).unwrap()
}

fn identity(low: u64, revision: u64, engine: Engine) -> PageIdentity {
    PageIdentity::new(result_id(low), Revision::from_wire_u64(revision), engine)
}

fn page(identity: PageIdentity, start: u64, value: &str) -> ResultPage {
    page_rows(identity, start, &[value])
}

fn page_rows(identity: PageIdentity, start: u64, values: &[&str]) -> ResultPage {
    let name = BoundedText::copy_from_str("value", ByteLimit::new(5)).unwrap();
    let type_name = BoundedText::copy_from_str("String", ByteLimit::new(6)).unwrap();
    let column = ColumnMetadata::new(
        name,
        EngineType::new(identity.engine(), type_name).unwrap(),
        false,
    );
    let values = values
        .iter()
        .map(|value| {
            OwnedValue::text(
                BoundedText::copy_from_str(value, ByteLimit::new(1024)).unwrap(),
                Truncation::Complete,
            )
            .unwrap()
        })
        .collect();
    ResultPage::from_row_major(
        identity,
        start,
        RowTotal::Unknown,
        PageFacts::new(PageDelivery::Partial, PageWarnings::none()),
        vec![column],
        values,
        PageLimits::new(8, 1, 1024, 64),
    )
    .unwrap()
}

fn limits(results: u32, pages: u32, bytes: u64) -> ResultStoreLimits {
    ResultStoreLimits::new(results, pages, bytes).unwrap()
}

#[test]
fn limits_are_finite_and_nonzero() {
    assert_eq!(
        ResultStoreLimits::new(0, 1, 1),
        Err(ResultStoreError::InvalidLimits)
    );
    assert_eq!(
        ResultStoreLimits::new(1, 0, 1),
        Err(ResultStoreError::InvalidLimits)
    );
    assert_eq!(
        ResultStoreLimits::new(1, 1, 0),
        Err(ResultStoreError::InvalidLimits)
    );
}

#[test]
fn pages_require_an_explicit_matching_open_result() {
    let current = identity(1, 3, Engine::PostgreSql);
    let mut store = ResultStore::new(limits(1, 2, 4096));
    assert!(matches!(
        store.admit(page(current, 0, "secret")),
        Err(ResultStoreError::ResultNotOpen)
    ));
    assert_eq!(store.open_result(current), Ok(OpenResultOutcome::Opened));
    assert_eq!(
        store.open_result(identity(1, 4, Engine::ClickHouse)),
        Err(ResultStoreError::EngineMismatch)
    );
    assert_eq!(
        store.admit(page(identity(1, 2, Engine::PostgreSql), 0, "old")),
        Err(ResultStoreError::StaleRevision)
    );
    assert_eq!(
        store.admit(page(identity(1, 4, Engine::PostgreSql), 0, "future")),
        Err(ResultStoreError::FutureRevision)
    );
    assert_eq!(
        store.admit(page(identity(1, 3, Engine::ClickHouse), 0, "wrong")),
        Err(ResultStoreError::EngineMismatch)
    );
    assert_eq!(store.page_count(), 0);
}

#[test]
fn eviction_is_global_lru_with_stable_page_key_ties() {
    let first_result = identity(1, 0, Engine::PostgreSql);
    let second_result = identity(2, 0, Engine::ClickHouse);
    let mut store = ResultStore::new(limits(2, 2, 16_384));
    store.open_result(first_result).unwrap();
    store.open_result(second_result).unwrap();
    let first = store
        .admit(page(first_result, 0, "first"))
        .unwrap()
        .admitted();
    let stale = store
        .admit(page(first_result, 10, "stale"))
        .unwrap()
        .admitted();
    assert!(store.get(first).is_some());

    let outcome = store.admit(page(second_result, 0, "new")).unwrap();
    assert_eq!(outcome.evicted(), &[stale]);
    assert!(store.get(stale).is_none());
    assert!(store.get(first).is_some());
    assert_eq!(store.page_count(), 2);
}

#[test]
fn pinned_capacity_failure_is_transactional() {
    let first_result = identity(1, 0, Engine::PostgreSql);
    let second_result = identity(2, 0, Engine::ClickHouse);
    let mut store = ResultStore::new(limits(2, 1, 4096));
    store.open_result(first_result).unwrap();
    store.open_result(second_result).unwrap();
    let first = store
        .admit(page(first_result, 0, "first"))
        .unwrap()
        .admitted();
    assert!(store.set_pinned(first, true));
    let bytes = store.resident_buffer_bytes();

    assert!(matches!(
        store.admit(page(second_result, 0, "second")),
        Err(ResultStoreError::PinnedCapacity)
    ));
    assert_eq!(store.page_count(), 1);
    assert_eq!(store.resident_buffer_bytes(), bytes);
    assert!(store.get(first).is_some());
}

#[test]
fn revision_replacement_invalidates_even_pinned_pages() {
    let old_identity = identity(1, 4, Engine::Redis);
    let new_identity = identity(1, 5, Engine::Redis);
    let mut store = ResultStore::new(limits(1, 2, 4096));
    store.open_result(old_identity).unwrap();
    let old = store
        .admit(page(old_identity, 0, "old"))
        .unwrap()
        .admitted();
    assert!(store.set_pinned(old, true));

    assert_eq!(
        store.open_result(new_identity),
        Ok(OpenResultOutcome::Replaced { evicted: vec![old] })
    );
    assert!(store.get(old).is_none());
    assert_eq!(store.page_count(), 0);
    assert_eq!(store.resident_buffer_bytes(), 0);
    assert_eq!(
        store.admit(page(old_identity, 0, "late")),
        Err(ResultStoreError::StaleRevision)
    );
    assert!(store.admit(page(new_identity, 0, "new")).is_ok());
}

#[test]
fn duplicate_overlap_slot_and_byte_limits_fail_without_mutation() {
    let first_identity = identity(1, 0, Engine::PostgreSql);
    let mut store = ResultStore::new(limits(1, 3, 4096));
    store.open_result(first_identity).unwrap();
    assert_eq!(
        store.open_result(identity(2, 0, Engine::Redis)),
        Err(ResultStoreError::ResultSlotsFull)
    );
    let admitted = store
        .admit(page_rows(first_identity, 10, &["resident", "second"]))
        .unwrap();
    assert_eq!(
        store.admit(page(first_identity, 10, "duplicate")),
        Err(ResultStoreError::DuplicatePage)
    );
    assert_eq!(
        store.admit(page(first_identity, 11, "overlap")),
        Err(ResultStoreError::OverlappingPage)
    );
    assert_eq!(store.page_count(), 1);

    let resident = store.get(admitted.admitted()).unwrap();
    let exact_bytes = resident.resident_buffer_bytes();
    assert!(exact_bytes > resident.cell(0, 0).unwrap().bytes().len() as u64);
    let closed = store.close_result(first_identity.result_id());
    assert_eq!(closed, vec![admitted.admitted()]);
    assert_eq!(store.resident_buffer_bytes(), 0);

    let mut too_small = ResultStore::new(limits(1, 1, exact_bytes - 1));
    too_small.open_result(first_identity).unwrap();
    assert!(matches!(
        too_small.admit(page_rows(first_identity, 0, &["resident", "second"])),
        Err(ResultStoreError::PageTooLarge { actual, limit })
            if actual == exact_bytes && limit == exact_bytes - 1
    ));
}

#[test]
fn debug_output_never_contains_page_values() {
    let identity = identity(1, 0, Engine::PostgreSql);
    let mut store = ResultStore::new(limits(1, 1, 4096));
    store.open_result(identity).unwrap();
    let AdmissionOutcome { .. } = store.admit(page(identity, 0, "do-not-log")).unwrap();
    let debug = format!("{store:?}");
    assert!(!debug.contains("do-not-log"));
    assert!(debug.contains("resident_buffer_bytes"));
}
