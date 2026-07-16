use tablerock_core::{
    BoundedText, ByteLimit, Engine, IdParts, ProfileGroupName, ProfileId, ProfileListError,
    ProfileListFilter, ProfileListItem, ProfileListPage, ProfileListRequest, ProfileName,
    ProfileSafetyMode, ProfileSourceFacts, PropertyValueSource, Revision,
};

fn text(value: &str) -> BoundedText {
    BoundedText::copy_from_str(value, ByteLimit::new(128)).unwrap()
}

fn profile_id(low: u64) -> ProfileId {
    ProfileId::from_parts(IdParts::new(1, low).unwrap()).unwrap()
}

fn item(low: u64) -> ProfileListItem {
    ProfileListItem::new(
        profile_id(low),
        Revision::from_wire_u64(low),
        Engine::PostgreSql,
        ProfileName::new(text("Private profile name")).unwrap(),
        Some(ProfileGroupName::new(text("Private group")).unwrap()),
        true,
        low as u32,
        ProfileSafetyMode::ReadOnly,
        ProfileSourceFacts::new(
            PropertyValueSource::Literal,
            PropertyValueSource::SecretSource,
            true,
            true,
        ),
    )
}

#[test]
fn request_and_page_are_bounded_and_cursor_based() {
    assert_eq!(
        ProfileListRequest::new(ProfileListFilter::default(), None, 0),
        Err(ProfileListError::InvalidLimit {
            actual: 0,
            maximum: 100,
        })
    );
    assert!(ProfileListRequest::new(ProfileListFilter::default(), None, 101).is_err());
    let request = ProfileListRequest::new(ProfileListFilter::default(), None, 2).unwrap();
    let page = ProfileListPage::new(&request, vec![item(1), item(2)], true).unwrap();
    assert_eq!(page.items().len(), 2);
    let next = page.next().unwrap();
    assert_eq!(next.id(), profile_id(2));
    assert_eq!(next.saved_order(), 2);
    assert!(next.favorite());
    assert!(matches!(
        ProfileListPage::new(&request, vec![item(1), item(2), item(3)], false),
        Err(ProfileListError::TooManyItems { .. })
    ));
    assert_eq!(
        ProfileListPage::new(&request, Vec::new(), true),
        Err(ProfileListError::EmptyContinuation)
    );
}

#[test]
fn continuation_is_bound_to_its_filter_scope() {
    let filter = ProfileListFilter::new(Some(Engine::Redis), Some(true))
        .with_group(Some(ProfileGroupName::new(text("Cache")).unwrap()))
        .with_tag(Some(
            tablerock_core::ProfileTag::new(text("primary")).unwrap(),
        ));
    let request = ProfileListRequest::new(filter.clone(), None, 1).unwrap();
    let page = ProfileListPage::new(&request, vec![item(1)], true).unwrap();
    let cursor = page.next().unwrap();
    assert_eq!(cursor.filter(), &filter);
    let debug = format!("{cursor:?}");
    assert!(!debug.contains("Cache"));
    assert!(!debug.contains("primary"));
    assert_eq!(
        ProfileListRequest::new(ProfileListFilter::default(), Some(cursor.clone()), 1),
        Err(ProfileListError::CursorFilterMismatch)
    );
    assert!(ProfileListRequest::new(filter, Some(cursor), 1).is_ok());

    let different_tag = ProfileListFilter::new(Some(Engine::Redis), Some(true))
        .with_group(Some(ProfileGroupName::new(text("Cache")).unwrap()))
        .with_tag(Some(
            tablerock_core::ProfileTag::new(text("replica")).unwrap(),
        ));
    assert!(matches!(
        ProfileListRequest::new(different_tag, page.next(), 1),
        Err(ProfileListError::CursorFilterMismatch)
    ));
}

#[test]
fn summaries_expose_source_facts_but_redact_labels() {
    let item = item(7);
    assert_eq!(item.name().as_str(), "Private profile name");
    assert_eq!(item.group().unwrap().as_str(), "Private group");
    assert_eq!(item.sources().host(), PropertyValueSource::Literal);
    assert_eq!(item.sources().port(), PropertyValueSource::SecretSource);
    assert!(item.sources().has_secret_sources());
    assert!(item.sources().has_dangerous_plaintext());
    let debug = format!("{item:?}");
    assert!(!debug.contains("Private profile name"));
    assert!(!debug.contains("Private group"));
}
