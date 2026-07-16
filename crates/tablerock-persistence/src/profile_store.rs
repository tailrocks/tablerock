use tablerock_core::{
    BoundedBytes, BoundedText, ByteLimit, DangerousPlaintext, DangerousTlsAcknowledgement, Engine,
    EnvironmentReference, KeychainReference, OnePasswordObjectId, OnePasswordReference,
    OnePasswordSegment, PersistableProfile, PlaintextAcknowledgement, ProfileAggregate,
    ProfileConnectionSnapshot, ProfileDurability, ProfileGroupName, ProfileId, ProfileIdentity,
    ProfileLimits, ProfileListItem, ProfileListPage, ProfileListRequest, ProfileName,
    ProfileOrganization, ProfilePolicy, ProfilePreferences, ProfileProperty,
    ProfilePropertyBinding, ProfilePropertySet, ProfileSafetyMode, ProfileSourceFacts, ProfileTag,
    PropertyValueSource, ReconnectPreference, Revision, SecretSource, SecretSourceKind, TlsPolicy,
};
use zeroize::Zeroize;

use crate::{PersistenceError, query_u32};

pub(crate) struct EncodedProfile {
    id: [u8; 16],
    aggregate_schema: u16,
    connection_schema: u16,
    property_schema: u16,
    revision: [u8; 8],
    engine: u8,
    name: String,
    tls_policy: u8,
    safety_mode: u8,
    connect_timeout_ms: u64,
    operation_timeout_ms: u64,
    max_result_rows: u64,
    max_result_bytes: u64,
    group_name: Option<String>,
    favorite: bool,
    saved_order: u32,
    reconnect: u8,
    restore_last_context: bool,
    preferred_page_rows: u32,
    tags: Vec<String>,
    properties: Vec<EncodedProperty>,
}

impl EncodedProfile {
    pub(crate) fn from_saved(saved: PersistableProfile<'_>) -> Self {
        let profile = saved.profile();
        let connection = profile.connection();
        let organization = profile.organization();
        let preferences = profile.preferences();
        let properties = ProfileProperty::ALL
            .iter()
            .filter_map(|property| connection.properties().binding(*property))
            .enumerate()
            .map(|(ordinal, binding)| EncodedProperty::new(ordinal, binding))
            .collect();
        Self {
            id: connection.id().to_bytes(),
            aggregate_schema: profile.schema_version(),
            connection_schema: connection.schema_version(),
            property_schema: connection.properties().schema_version(),
            revision: connection.revision().get().to_be_bytes(),
            engine: encode_engine(connection.engine()),
            name: connection.name().as_str().to_owned(),
            tls_policy: encode_tls(connection.tls_policy()),
            safety_mode: encode_safety(connection.safety_mode()),
            connect_timeout_ms: connection.limits().connect_timeout_ms(),
            operation_timeout_ms: connection.limits().operation_timeout_ms(),
            max_result_rows: connection.limits().max_result_rows(),
            max_result_bytes: connection.limits().max_result_bytes(),
            group_name: organization.group().map(|group| group.as_str().to_owned()),
            favorite: organization.favorite(),
            saved_order: organization.order(),
            reconnect: match preferences.reconnect() {
                ReconnectPreference::Manual => 1,
                ReconnectPreference::BoundedAutomatic => 2,
            },
            restore_last_context: preferences.restore_last_context(),
            preferred_page_rows: preferences.preferred_page_rows(),
            tags: organization
                .tags()
                .iter()
                .map(|tag| tag.as_str().to_owned())
                .collect(),
            properties,
        }
    }
}

struct EncodedProperty {
    ordinal: u8,
    property: u8,
    source_kind: u8,
    source_schema: Option<u16>,
    text_value: Option<String>,
    blob_value: Option<SensitiveBlob>,
    op_account_id: Option<String>,
    op_vault_id: Option<String>,
    op_item_id: Option<String>,
    op_section_id: Option<String>,
    op_field_id: Option<String>,
    op_breadcrumb: Option<String>,
}

impl EncodedProperty {
    fn new(ordinal: usize, binding: &tablerock_core::ProfilePropertyBinding) -> Self {
        let mut encoded = Self {
            ordinal: ordinal as u8,
            property: encode_property(binding.property()),
            source_kind: 1,
            source_schema: None,
            text_value: None,
            blob_value: None,
            op_account_id: None,
            op_vault_id: None,
            op_item_id: None,
            op_section_id: None,
            op_field_id: None,
            op_breadcrumb: None,
        };
        match binding.source() {
            PropertyValueSource::Literal => {
                encoded.text_value = binding.literal_value().map(str::to_owned);
            }
            PropertyValueSource::SecretSource => {
                let secret = binding.secret_source().expect("source kind is secret");
                encoded.source_schema = Some(secret.schema_version());
                match secret.kind() {
                    SecretSourceKind::OnePassword(reference) => {
                        encoded.source_kind = 2;
                        encoded.op_account_id = Some(reference.account_id().as_str().to_owned());
                        encoded.op_vault_id = Some(reference.vault_id().as_str().to_owned());
                        encoded.op_item_id = Some(reference.item_id().as_str().to_owned());
                        encoded.op_section_id = reference
                            .section_id()
                            .map(|value| value.as_str().to_owned());
                        encoded.op_field_id = Some(reference.field_id().as_str().to_owned());
                        encoded.op_breadcrumb = Some(reference.breadcrumb().to_owned());
                    }
                    SecretSourceKind::PromptOnConnect => encoded.source_kind = 3,
                    SecretSourceKind::HostEnvironment(reference) => {
                        encoded.source_kind = 4;
                        encoded.text_value = Some(reference.as_str().to_owned());
                    }
                    SecretSourceKind::Keychain(reference) => {
                        encoded.source_kind = 5;
                        encoded.blob_value = Some(SensitiveBlob(reference.bytes().to_vec()));
                    }
                    SecretSourceKind::DangerousPlaintext(value) => {
                        encoded.source_kind = 6;
                        encoded.blob_value = Some(SensitiveBlob(value.bytes().to_vec()));
                    }
                }
            }
        }
        encoded
    }
}

struct SensitiveBlob(Vec<u8>);

impl SensitiveBlob {
    fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for SensitiveBlob {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

pub(crate) async fn create(
    connection: &mut turso::Connection,
    profile: &EncodedProfile,
) -> Result<(), PersistenceError> {
    let exists = query_u32(
        connection,
        "SELECT COUNT(*) FROM saved_profiles WHERE profile_id = ?1",
        (profile.id.as_slice(),),
    )
    .await?;
    if exists != 0 {
        return Err(PersistenceError::ProfileAlreadyExists);
    }
    let count = query_u32(connection, "SELECT COUNT(*) FROM saved_profiles", ()).await?;
    validate_profile_capacity(count)?;
    let transaction = connection
        .transaction()
        .await
        .map_err(|_| PersistenceError::ProfileWrite)?;
    transaction
        .execute(
            "INSERT INTO saved_profiles(\
                profile_id, aggregate_schema, connection_schema, property_schema, revision,\
                engine, name, tls_policy, safety_mode, connect_timeout_ms,\
                operation_timeout_ms, max_result_rows, max_result_bytes, group_name, favorite,\
                saved_order, reconnect, restore_last_context, preferred_page_rows\
             ) VALUES (\
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,\
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19\
             )",
            turso::params![
                profile.id.as_slice(),
                profile.aggregate_schema,
                profile.connection_schema,
                profile.property_schema,
                profile.revision.as_slice(),
                profile.engine,
                profile.name.as_str(),
                profile.tls_policy,
                profile.safety_mode,
                profile.connect_timeout_ms,
                profile.operation_timeout_ms,
                profile.max_result_rows,
                profile.max_result_bytes,
                profile.group_name.as_deref(),
                profile.favorite,
                profile.saved_order,
                profile.reconnect,
                profile.restore_last_context,
                profile.preferred_page_rows,
            ],
        )
        .await
        .map_err(|_| PersistenceError::ProfileWrite)?;
    for (ordinal, tag) in profile.tags.iter().enumerate() {
        transaction
            .execute(
                "INSERT INTO saved_profile_tags(profile_id, ordinal, tag) VALUES (?1, ?2, ?3)",
                (profile.id.as_slice(), ordinal as u8, tag.as_str()),
            )
            .await
            .map_err(|_| PersistenceError::ProfileWrite)?;
    }
    for property in &profile.properties {
        transaction
            .execute(
                "INSERT INTO saved_profile_properties(\
                    profile_id, ordinal, property, source_kind, source_schema, text_value,\
                    blob_value, op_account_id, op_vault_id, op_item_id, op_section_id,\
                    op_field_id, op_breadcrumb\
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                turso::params![
                    profile.id.as_slice(),
                    property.ordinal,
                    property.property,
                    property.source_kind,
                    property.source_schema,
                    property.text_value.as_deref(),
                    property.blob_value.as_ref().map(SensitiveBlob::as_slice),
                    property.op_account_id.as_deref(),
                    property.op_vault_id.as_deref(),
                    property.op_item_id.as_deref(),
                    property.op_section_id.as_deref(),
                    property.op_field_id.as_deref(),
                    property.op_breadcrumb.as_deref(),
                ],
            )
            .await
            .map_err(|_| PersistenceError::ProfileWrite)?;
    }
    transaction
        .commit()
        .await
        .map_err(|_| PersistenceError::ProfileWrite)
}

fn validate_profile_capacity(count: u32) -> Result<(), PersistenceError> {
    if count >= ProfileListRequest::MAX_SEARCH_CANDIDATES as u32 {
        Err(PersistenceError::ProfileCapacity)
    } else {
        Ok(())
    }
}

pub(crate) async fn replace(
    connection: &mut turso::Connection,
    expected_revision: Revision,
    profile: &EncodedProfile,
) -> Result<(), PersistenceError> {
    let next = expected_revision
        .checked_next()
        .map_err(|_| PersistenceError::ProfileInvalidRevision)?;
    if profile.revision != next.get().to_be_bytes() {
        return Err(PersistenceError::ProfileInvalidRevision);
    }
    let transaction = connection
        .transaction()
        .await
        .map_err(|_| PersistenceError::ProfileWrite)?;
    let changed = transaction
        .execute(
            "UPDATE saved_profiles SET \
                aggregate_schema = ?1, connection_schema = ?2, property_schema = ?3, \
                revision = ?4, engine = ?5, name = ?6, tls_policy = ?7, safety_mode = ?8, \
                connect_timeout_ms = ?9, operation_timeout_ms = ?10, max_result_rows = ?11, \
                max_result_bytes = ?12, group_name = ?13, favorite = ?14, saved_order = ?15, \
                reconnect = ?16, restore_last_context = ?17, preferred_page_rows = ?18, \
                updated_at = CURRENT_TIMESTAMP \
             WHERE profile_id = ?19 AND revision = ?20",
            turso::params![
                profile.aggregate_schema,
                profile.connection_schema,
                profile.property_schema,
                profile.revision.as_slice(),
                profile.engine,
                profile.name.as_str(),
                profile.tls_policy,
                profile.safety_mode,
                profile.connect_timeout_ms,
                profile.operation_timeout_ms,
                profile.max_result_rows,
                profile.max_result_bytes,
                profile.group_name.as_deref(),
                profile.favorite,
                profile.saved_order,
                profile.reconnect,
                profile.restore_last_context,
                profile.preferred_page_rows,
                profile.id.as_slice(),
                expected_revision.get().to_be_bytes().as_slice(),
            ],
        )
        .await
        .map_err(|_| PersistenceError::ProfileWrite)?;
    if changed == 0 {
        let exists = query_u32(
            &transaction,
            "SELECT COUNT(*) FROM saved_profiles WHERE profile_id = ?1",
            (profile.id.as_slice(),),
        )
        .await?;
        let _ = transaction.rollback().await;
        return Err(if exists == 0 {
            PersistenceError::ProfileNotFound
        } else {
            PersistenceError::ProfileStaleRevision
        });
    }
    transaction
        .execute(
            "DELETE FROM saved_profile_tags WHERE profile_id = ?1",
            (profile.id.as_slice(),),
        )
        .await
        .map_err(|_| PersistenceError::ProfileWrite)?;
    transaction
        .execute(
            "DELETE FROM saved_profile_properties WHERE profile_id = ?1",
            (profile.id.as_slice(),),
        )
        .await
        .map_err(|_| PersistenceError::ProfileWrite)?;
    insert_children(&transaction, profile).await?;
    transaction
        .commit()
        .await
        .map_err(|_| PersistenceError::ProfileWrite)
}

pub(crate) async fn delete(
    connection: &mut turso::Connection,
    id: ProfileId,
    expected_revision: Revision,
) -> Result<(), PersistenceError> {
    let transaction = connection
        .transaction()
        .await
        .map_err(|_| PersistenceError::ProfileWrite)?;
    let changed = transaction
        .execute(
            "DELETE FROM saved_profiles WHERE profile_id = ?1 AND revision = ?2",
            (
                id.to_bytes().as_slice(),
                expected_revision.get().to_be_bytes().as_slice(),
            ),
        )
        .await
        .map_err(|_| PersistenceError::ProfileWrite)?;
    if changed == 0 {
        let exists = query_u32(
            &transaction,
            "SELECT COUNT(*) FROM saved_profiles WHERE profile_id = ?1",
            (id.to_bytes().as_slice(),),
        )
        .await?;
        let _ = transaction.rollback().await;
        return Err(if exists == 0 {
            PersistenceError::ProfileNotFound
        } else {
            PersistenceError::ProfileStaleRevision
        });
    }
    transaction
        .commit()
        .await
        .map_err(|_| PersistenceError::ProfileWrite)
}

pub(crate) async fn list(
    connection: &mut turso::Connection,
    request: ProfileListRequest,
) -> Result<ProfileListPage, PersistenceError> {
    let transaction = connection
        .transaction()
        .await
        .map_err(|_| PersistenceError::ProfileRead)?;
    match list_transaction(&transaction, &request).await {
        Ok(page) => {
            transaction
                .commit()
                .await
                .map_err(|_| PersistenceError::ProfileRead)?;
            Ok(page)
        }
        Err(error) => {
            let _ = transaction.rollback().await;
            Err(error)
        }
    }
}

async fn list_transaction(
    connection: &turso::Connection,
    request: &ProfileListRequest,
) -> Result<ProfileListPage, PersistenceError> {
    const PROJECTION: &str = "SELECT saved_profiles.profile_id, revision, engine, name, group_name, favorite, \
        saved_order, safety_mode, \
        (SELECT source_kind FROM saved_profile_properties p \
         WHERE p.profile_id = saved_profiles.profile_id AND property = 1), \
        (SELECT source_kind FROM saved_profile_properties p \
         WHERE p.profile_id = saved_profiles.profile_id AND property = 2), \
        EXISTS(SELECT 1 FROM saved_profile_properties p \
               WHERE p.profile_id = saved_profiles.profile_id AND source_kind > 1), \
        EXISTS(SELECT 1 FROM saved_profile_properties p \
               WHERE p.profile_id = saved_profiles.profile_id AND source_kind = 6)";
    let search = request.filter().search();
    let projection = if search.is_some() {
        format!(
            "{PROJECTION}, (SELECT group_concat(tag, char(31)) FROM saved_profile_tags search_tag \
             WHERE search_tag.profile_id = saved_profiles.profile_id) "
        )
    } else {
        format!("{PROJECTION} ")
    };
    let fetch_limit = if search.is_some() {
        ProfileListRequest::MAX_SEARCH_CANDIDATES as u32 + 1
    } else {
        u32::from(request.limit()) + 1
    };
    let mut conditions = Vec::with_capacity(5);
    let mut parameters = Vec::with_capacity(9);
    let filter = request.filter();
    if let Some(engine) = filter.engine() {
        let parameter = push_parameter(&mut parameters, i64::from(encode_engine(engine)));
        conditions.push(format!("engine = {parameter}"));
    }
    if let Some(favorite) = filter.favorite() {
        let parameter = push_parameter(&mut parameters, i64::from(u8::from(favorite)));
        conditions.push(format!("favorite = {parameter}"));
    }
    if let Some(group) = filter.group() {
        let parameter = push_parameter(&mut parameters, group.as_str().to_owned());
        conditions.push(format!("group_name = {parameter}"));
    }
    let from_clause = if let Some(tag) = filter.tag() {
        let parameter = push_parameter(&mut parameters, tag.as_str().to_owned());
        conditions.push(format!("filtered_tag.tag = {parameter}"));
        "FROM saved_profile_tags filtered_tag \
         JOIN saved_profiles ON saved_profiles.profile_id = filtered_tag.profile_id "
    } else {
        "FROM saved_profiles "
    };
    if let Some(after) = request.after() {
        let favorite = push_parameter(&mut parameters, i64::from(u8::from(after.favorite())));
        let order = push_parameter(&mut parameters, i64::from(after.saved_order()));
        let id = push_parameter(&mut parameters, after.id().to_bytes().to_vec());
        conditions.push(format!(
            "(favorite < {favorite} OR (favorite = {favorite} AND \
             (saved_order > {order} OR (saved_order = {order} AND saved_profiles.profile_id > {id}))))"
        ));
    }
    let limit = push_parameter(&mut parameters, i64::from(fetch_limit));
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {} ", conditions.join(" AND "))
    };
    let sql = format!(
        "{projection}{from_clause}{where_clause}ORDER BY favorite DESC, saved_order, saved_profiles.profile_id LIMIT {limit}"
    );
    let mut rows = connection
        .query(sql, parameters)
        .await
        .map_err(|_| PersistenceError::ProfileRead)?;
    let mut items = Vec::with_capacity(usize::from(request.limit()) + 1);
    let mut candidate_count = 0usize;
    while let Some(row) = rows
        .next()
        .await
        .map_err(|_| PersistenceError::ProfileRead)?
    {
        candidate_count += 1;
        if candidate_count > ProfileListRequest::MAX_SEARCH_CANDIDATES {
            return Err(PersistenceError::ProfileCapacity);
        }
        let item = decode_list_item(&row)?;
        let matches = if let Some(search) = search {
            let tags = decode_search_tags(&row)?;
            search.matches(item.name().as_str())
                || item
                    .group()
                    .is_some_and(|group| search.matches(group.as_str()))
                || tags.iter().any(|tag| search.matches(tag.as_str()))
        } else {
            true
        };
        if matches && items.len() <= usize::from(request.limit()) {
            items.push(item);
        }
    }
    drop(rows);
    let has_more = items.len() > usize::from(request.limit());
    items.truncate(usize::from(request.limit()));
    ProfileListPage::new(request, items, has_more).map_err(|_| PersistenceError::ProfileDecode)
}

fn decode_search_tags(row: &turso::Row) -> Result<Vec<ProfileTag>, PersistenceError> {
    let Some(tags) = get::<Option<String>>(row, 12)? else {
        return Ok(Vec::new());
    };
    let mut decoded = Vec::new();
    for tag in tags.split('\u{1f}') {
        if decoded.len() >= ProfileOrganization::MAX_TAGS {
            return Err(PersistenceError::ProfileDecode);
        }
        decoded.push(
            ProfileTag::new(bounded_text(tag.to_owned(), ProfileTag::MAX_BYTES)?)
                .map_err(|_| PersistenceError::ProfileDecode)?,
        );
    }
    Ok(decoded)
}

fn push_parameter(parameters: &mut Vec<turso::Value>, value: impl Into<turso::Value>) -> String {
    parameters.push(value.into());
    format!("?{}", parameters.len())
}

fn decode_list_item(row: &turso::Row) -> Result<ProfileListItem, PersistenceError> {
    let id = ProfileId::from_bytes(get::<[u8; 16]>(row, 0)?)
        .map_err(|_| PersistenceError::ProfileDecode)?;
    let revision = Revision::from_wire_u64(u64::from_be_bytes(get::<[u8; 8]>(row, 1)?));
    let engine = decode_engine(get::<u8>(row, 2)?)?;
    let name = ProfileName::new(bounded_text(
        get::<String>(row, 3)?,
        ProfileName::MAX_BYTES,
    )?)
    .map_err(|_| PersistenceError::ProfileDecode)?;
    let group = get::<Option<String>>(row, 4)?
        .map(|value| {
            ProfileGroupName::new(bounded_text(value, ProfileGroupName::MAX_BYTES)?)
                .map_err(|_| PersistenceError::ProfileDecode)
        })
        .transpose()?;
    let favorite = decode_bool(get::<u8>(row, 5)?)?;
    let saved_order = get::<u32>(row, 6)?;
    let safety_mode = decode_safety(get::<u8>(row, 7)?)?;
    let sources = ProfileSourceFacts::new(
        decode_property_source(get::<u8>(row, 8)?)?,
        decode_property_source(get::<u8>(row, 9)?)?,
        decode_bool(get::<u8>(row, 10)?)?,
        decode_bool(get::<u8>(row, 11)?)?,
    );
    Ok(ProfileListItem::new(
        id,
        revision,
        engine,
        name,
        group,
        favorite,
        saved_order,
        safety_mode,
        sources,
    ))
}

async fn insert_children(
    connection: &turso::Connection,
    profile: &EncodedProfile,
) -> Result<(), PersistenceError> {
    for (ordinal, tag) in profile.tags.iter().enumerate() {
        connection
            .execute(
                "INSERT INTO saved_profile_tags(profile_id, ordinal, tag) VALUES (?1, ?2, ?3)",
                (profile.id.as_slice(), ordinal as u8, tag.as_str()),
            )
            .await
            .map_err(|_| PersistenceError::ProfileWrite)?;
    }
    for property in &profile.properties {
        connection
            .execute(
                "INSERT INTO saved_profile_properties(\
                    profile_id, ordinal, property, source_kind, source_schema, text_value,\
                    blob_value, op_account_id, op_vault_id, op_item_id, op_section_id,\
                    op_field_id, op_breadcrumb\
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                turso::params![
                    profile.id.as_slice(),
                    property.ordinal,
                    property.property,
                    property.source_kind,
                    property.source_schema,
                    property.text_value.as_deref(),
                    property.blob_value.as_ref().map(SensitiveBlob::as_slice),
                    property.op_account_id.as_deref(),
                    property.op_vault_id.as_deref(),
                    property.op_item_id.as_deref(),
                    property.op_section_id.as_deref(),
                    property.op_field_id.as_deref(),
                    property.op_breadcrumb.as_deref(),
                ],
            )
            .await
            .map_err(|_| PersistenceError::ProfileWrite)?;
    }
    Ok(())
}

pub(crate) async fn read(
    connection: &mut turso::Connection,
    id: ProfileId,
) -> Result<Option<ProfileAggregate>, PersistenceError> {
    let transaction = connection
        .transaction()
        .await
        .map_err(|_| PersistenceError::ProfileRead)?;
    match read_transaction(&transaction, id).await {
        Ok(profile) => {
            transaction
                .commit()
                .await
                .map_err(|_| PersistenceError::ProfileRead)?;
            Ok(profile)
        }
        Err(error) => {
            // Preserve the fail-closed decode result. Dropping the consumed transaction
            // remains a rollback boundary even if the driver cannot report rollback while
            // a failed row decoder is unwinding an active statement.
            let _ = transaction.rollback().await;
            Err(error)
        }
    }
}

async fn read_transaction(
    connection: &turso::Connection,
    id: ProfileId,
) -> Result<Option<ProfileAggregate>, PersistenceError> {
    let mut rows = connection
        .query(
            "SELECT aggregate_schema, connection_schema, property_schema, revision, engine,\
                    name, tls_policy, safety_mode, connect_timeout_ms, operation_timeout_ms,\
                    max_result_rows, max_result_bytes, group_name, favorite, saved_order,\
                    reconnect, restore_last_context, preferred_page_rows \
             FROM saved_profiles WHERE profile_id = ?1",
            (id.to_bytes().as_slice(),),
        )
        .await
        .map_err(|_| PersistenceError::ProfileRead)?;
    let Some(row) = rows
        .next()
        .await
        .map_err(|_| PersistenceError::ProfileRead)?
    else {
        return Ok(None);
    };
    let aggregate_schema = get::<u16>(&row, 0)?;
    let connection_schema = get::<u16>(&row, 1)?;
    let property_schema = get::<u16>(&row, 2)?;
    let revision = Revision::from_wire_u64(u64::from_be_bytes(get::<[u8; 8]>(&row, 3)?));
    let engine = decode_engine(get::<u8>(&row, 4)?)?;
    let name = ProfileName::new(bounded_text(get::<String>(&row, 5)?, 128)?)
        .map_err(|_| PersistenceError::ProfileDecode)?;
    let tls_policy = decode_tls(get::<u8>(&row, 6)?)?;
    let safety_mode = decode_safety(get::<u8>(&row, 7)?)?;
    let limits = ProfileLimits::new(
        get::<u64>(&row, 8)?,
        get::<u64>(&row, 9)?,
        get::<u64>(&row, 10)?,
        get::<u64>(&row, 11)?,
    )
    .map_err(|_| PersistenceError::ProfileDecode)?;
    let group_name = get::<Option<String>>(&row, 12)?
        .map(|value| {
            ProfileGroupName::new(bounded_text(value, ProfileGroupName::MAX_BYTES)?)
                .map_err(|_| PersistenceError::ProfileDecode)
        })
        .transpose()?;
    let favorite = decode_bool(get::<u8>(&row, 13)?)?;
    let saved_order = get::<u32>(&row, 14)?;
    let reconnect = decode_reconnect(get::<u8>(&row, 15)?)?;
    let restore_last_context = decode_bool(get::<u8>(&row, 16)?)?;
    let preferred_page_rows = get::<u32>(&row, 17)?;
    drop(row);
    drop(rows);

    let tags = read_tags(connection, id).await?;
    let properties = read_properties(connection, id, property_schema).await?;
    let identity = ProfileIdentity::new(id, revision, engine, name);
    let connection = ProfileConnectionSnapshot::from_wire(
        connection_schema,
        identity,
        properties,
        ProfilePolicy::new(tls_policy, safety_mode, limits),
    )
    .map_err(|_| PersistenceError::ProfileDecode)?;
    let organization = ProfileOrganization::new(group_name, tags, favorite, saved_order)
        .map_err(|_| PersistenceError::ProfileDecode)?;
    let preferences = ProfilePreferences::new(reconnect, restore_last_context, preferred_page_rows)
        .map_err(|_| PersistenceError::ProfileDecode)?;
    ProfileAggregate::from_wire(
        aggregate_schema,
        connection,
        ProfileDurability::Saved,
        organization,
        preferences,
    )
    .map(Some)
    .map_err(|_| PersistenceError::ProfileDecode)
}

async fn read_tags(
    connection: &turso::Connection,
    id: ProfileId,
) -> Result<Vec<ProfileTag>, PersistenceError> {
    let mut rows = connection
        .query(
            "SELECT ordinal, tag FROM saved_profile_tags \
             WHERE profile_id = ?1 ORDER BY ordinal",
            (id.to_bytes().as_slice(),),
        )
        .await
        .map_err(|_| PersistenceError::ProfileRead)?;
    let mut tags = Vec::new();
    while let Some(row) = rows
        .next()
        .await
        .map_err(|_| PersistenceError::ProfileRead)?
    {
        if get::<usize>(&row, 0)? != tags.len() || tags.len() >= ProfileOrganization::MAX_TAGS {
            return Err(PersistenceError::ProfileDecode);
        }
        let value = bounded_text(get::<String>(&row, 1)?, ProfileTag::MAX_BYTES)?;
        tags.push(ProfileTag::new(value).map_err(|_| PersistenceError::ProfileDecode)?);
    }
    Ok(tags)
}

async fn read_properties(
    connection: &turso::Connection,
    id: ProfileId,
    schema_version: u16,
) -> Result<ProfilePropertySet, PersistenceError> {
    let mut rows = connection
        .query(
            "SELECT ordinal, property, source_kind, source_schema, text_value, blob_value,\
                    op_account_id, op_vault_id, op_item_id, op_section_id, op_field_id,\
                    op_breadcrumb \
             FROM saved_profile_properties WHERE profile_id = ?1 ORDER BY ordinal",
            (id.to_bytes().as_slice(),),
        )
        .await
        .map_err(|_| PersistenceError::ProfileRead)?;
    let mut bindings = Vec::new();
    while let Some(row) = rows
        .next()
        .await
        .map_err(|_| PersistenceError::ProfileRead)?
    {
        if get::<usize>(&row, 0)? != bindings.len()
            || bindings.len() >= ProfilePropertySet::MAX_BINDINGS
        {
            return Err(PersistenceError::ProfileDecode);
        }
        bindings.push(decode_binding(&row)?);
    }
    ProfilePropertySet::from_wire(schema_version, bindings)
        .map_err(|_| PersistenceError::ProfileDecode)
}

fn decode_binding(row: &turso::Row) -> Result<ProfilePropertyBinding, PersistenceError> {
    let property = decode_property(get::<u8>(row, 1)?)?;
    let source_kind = get::<u8>(row, 2)?;
    if source_kind == 1 {
        let value = get::<String>(row, 4)?;
        let value = bounded_text(value, property.literal_byte_limit())?;
        return ProfilePropertyBinding::literal(property, value)
            .map_err(|_| PersistenceError::ProfileDecode);
    }
    let schema_version = get::<u16>(row, 3)?;
    let kind = match source_kind {
        2 => SecretSourceKind::OnePassword(
            OnePasswordReference::new(
                OnePasswordObjectId::parse(&get::<String>(row, 6)?)
                    .map_err(|_| PersistenceError::ProfileDecode)?,
                OnePasswordObjectId::parse(&get::<String>(row, 7)?)
                    .map_err(|_| PersistenceError::ProfileDecode)?,
                OnePasswordObjectId::parse(&get::<String>(row, 8)?)
                    .map_err(|_| PersistenceError::ProfileDecode)?,
                get::<Option<String>>(row, 9)?
                    .map(|value| {
                        OnePasswordSegment::parse(&value)
                            .map_err(|_| PersistenceError::ProfileDecode)
                    })
                    .transpose()?,
                OnePasswordSegment::parse(&get::<String>(row, 10)?)
                    .map_err(|_| PersistenceError::ProfileDecode)?,
                bounded_text(
                    get::<String>(row, 11)?,
                    OnePasswordReference::MAX_BREADCRUMB_BYTES,
                )?,
            )
            .map_err(|_| PersistenceError::ProfileDecode)?,
        ),
        3 => SecretSourceKind::PromptOnConnect,
        4 => SecretSourceKind::HostEnvironment(
            EnvironmentReference::parse(&get::<String>(row, 4)?)
                .map_err(|_| PersistenceError::ProfileDecode)?,
        ),
        5 => SecretSourceKind::Keychain(
            KeychainReference::new(bounded_bytes(
                get::<Vec<u8>>(row, 5)?,
                KeychainReference::MAX_BYTES,
            )?)
            .map_err(|_| PersistenceError::ProfileDecode)?,
        ),
        6 => SecretSourceKind::DangerousPlaintext(
            DangerousPlaintext::new(
                get::<Vec<u8>>(row, 5)?,
                PlaintextAcknowledgement::LocalTestingOnly,
            )
            .map_err(|_| PersistenceError::ProfileDecode)?,
        ),
        _ => return Err(PersistenceError::ProfileDecode),
    };
    let source = SecretSource::from_wire(schema_version, kind)
        .map_err(|_| PersistenceError::ProfileDecode)?;
    Ok(ProfilePropertyBinding::secret(property, source))
}

fn get<T>(row: &turso::Row, index: usize) -> Result<T, PersistenceError>
where
    T: DecodeCell,
{
    let value = row
        .get_value(index)
        .map_err(|_| PersistenceError::ProfileDecode)?;
    T::decode(value).ok_or(PersistenceError::ProfileDecode)
}

trait DecodeCell: Sized {
    fn decode(value: turso::Value) -> Option<Self>;
}

macro_rules! unsigned_cell {
    ($type:ty) => {
        impl DecodeCell for $type {
            fn decode(value: turso::Value) -> Option<Self> {
                match value {
                    turso::Value::Integer(value) => Self::try_from(value).ok(),
                    _ => None,
                }
            }
        }
    };
}

unsigned_cell!(u8);
unsigned_cell!(u16);
unsigned_cell!(u32);
unsigned_cell!(u64);
unsigned_cell!(usize);

impl DecodeCell for String {
    fn decode(value: turso::Value) -> Option<Self> {
        match value {
            turso::Value::Text(value) => Some(value),
            _ => None,
        }
    }
}

impl DecodeCell for Option<String> {
    fn decode(value: turso::Value) -> Option<Self> {
        match value {
            turso::Value::Null => Some(None),
            turso::Value::Text(value) => Some(Some(value)),
            _ => None,
        }
    }
}

impl DecodeCell for Vec<u8> {
    fn decode(value: turso::Value) -> Option<Self> {
        match value {
            turso::Value::Blob(value) => Some(value),
            _ => None,
        }
    }
}

impl<const N: usize> DecodeCell for [u8; N] {
    fn decode(value: turso::Value) -> Option<Self> {
        match value {
            turso::Value::Blob(value) => value.try_into().ok(),
            _ => None,
        }
    }
}

fn bounded_text(value: String, maximum: u64) -> Result<BoundedText, PersistenceError> {
    BoundedText::from_string(value, ByteLimit::new(maximum))
        .map_err(|_| PersistenceError::ProfileDecode)
}

fn bounded_bytes(value: Vec<u8>, maximum: u64) -> Result<BoundedBytes, PersistenceError> {
    BoundedBytes::from_vec(value, ByteLimit::new(maximum))
        .map_err(|_| PersistenceError::ProfileDecode)
}

const fn decode_engine(value: u8) -> Result<Engine, PersistenceError> {
    match value {
        1 => Ok(Engine::PostgreSql),
        2 => Ok(Engine::ClickHouse),
        3 => Ok(Engine::Redis),
        _ => Err(PersistenceError::ProfileDecode),
    }
}

const fn decode_tls(value: u8) -> Result<TlsPolicy, PersistenceError> {
    match value {
        1 => Ok(TlsPolicy::Disabled),
        2 => Ok(TlsPolicy::VerifySystemRoots),
        3 => Ok(TlsPolicy::VerifyCustomCa),
        4 => Ok(TlsPolicy::DangerousAcceptInvalidCertificate(
            DangerousTlsAcknowledgement::LocalTestingOnly,
        )),
        _ => Err(PersistenceError::ProfileDecode),
    }
}

const fn decode_safety(value: u8) -> Result<ProfileSafetyMode, PersistenceError> {
    match value {
        1 => Ok(ProfileSafetyMode::ReadOnly),
        2 => Ok(ProfileSafetyMode::ConfirmWrites),
        _ => Err(PersistenceError::ProfileDecode),
    }
}

const fn decode_reconnect(value: u8) -> Result<ReconnectPreference, PersistenceError> {
    match value {
        1 => Ok(ReconnectPreference::Manual),
        2 => Ok(ReconnectPreference::BoundedAutomatic),
        _ => Err(PersistenceError::ProfileDecode),
    }
}

const fn decode_bool(value: u8) -> Result<bool, PersistenceError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(PersistenceError::ProfileDecode),
    }
}

const fn decode_property_source(value: u8) -> Result<PropertyValueSource, PersistenceError> {
    match value {
        1 => Ok(PropertyValueSource::Literal),
        2..=6 => Ok(PropertyValueSource::SecretSource),
        _ => Err(PersistenceError::ProfileDecode),
    }
}

const fn decode_property(value: u8) -> Result<ProfileProperty, PersistenceError> {
    match value {
        1 => Ok(ProfileProperty::Host),
        2 => Ok(ProfileProperty::Port),
        3 => Ok(ProfileProperty::DefaultContext),
        4 => Ok(ProfileProperty::Username),
        5 => Ok(ProfileProperty::Password),
        6 => Ok(ProfileProperty::TlsServerName),
        7 => Ok(ProfileProperty::TlsCaCertificate),
        8 => Ok(ProfileProperty::TlsClientCertificate),
        9 => Ok(ProfileProperty::TlsClientPrivateKey),
        10 => Ok(ProfileProperty::TlsClientPrivateKeyPassword),
        _ => Err(PersistenceError::ProfileDecode),
    }
}

const fn encode_engine(engine: Engine) -> u8 {
    match engine {
        Engine::PostgreSql => 1,
        Engine::ClickHouse => 2,
        Engine::Redis => 3,
    }
}

const fn encode_tls(policy: TlsPolicy) -> u8 {
    match policy {
        TlsPolicy::Disabled => 1,
        TlsPolicy::VerifySystemRoots => 2,
        TlsPolicy::VerifyCustomCa => 3,
        TlsPolicy::DangerousAcceptInvalidCertificate(_) => 4,
    }
}

const fn encode_safety(mode: ProfileSafetyMode) -> u8 {
    match mode {
        ProfileSafetyMode::ReadOnly => 1,
        ProfileSafetyMode::ConfirmWrites => 2,
    }
}

const fn encode_property(property: ProfileProperty) -> u8 {
    match property {
        ProfileProperty::Host => 1,
        ProfileProperty::Port => 2,
        ProfileProperty::DefaultContext => 3,
        ProfileProperty::Username => 4,
        ProfileProperty::Password => 5,
        ProfileProperty::TlsServerName => 6,
        ProfileProperty::TlsCaCertificate => 7,
        ProfileProperty::TlsClientCertificate => 8,
        ProfileProperty::TlsClientPrivateKey => 9,
        ProfileProperty::TlsClientPrivateKeyPassword => 10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_profile_capacity_bounds_search_work() {
        assert!(validate_profile_capacity(9_999).is_ok());
        assert_eq!(
            validate_profile_capacity(10_000),
            Err(PersistenceError::ProfileCapacity)
        );
        assert_eq!(
            validate_profile_capacity(u32::MAX),
            Err(PersistenceError::ProfileCapacity)
        );
    }
}
