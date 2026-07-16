use tablerock_core::{
    Engine, PersistableProfile, ProfileProperty, ProfileSafetyMode, PropertyValueSource,
    ReconnectPreference, SecretSourceKind, TlsPolicy,
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
