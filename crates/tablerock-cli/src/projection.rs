//! Map core/persistence facts into TUI presentation projections.

use tablerock_core::{
    Engine, EnvironmentTag, ProfileEndpointPart, ProfileListItem, ProfileSafetyMode,
};
use tablerock_tui::{LiveConnectionState, ProfileRowProjection};

#[must_use]
pub fn profile_row(item: &ProfileListItem) -> ProfileRowProjection {
    let endpoint = item.endpoint();
    let host = match endpoint.host() {
        ProfileEndpointPart::Literal(value) => value.as_str().to_owned(),
        ProfileEndpointPart::SecretSource => "•".to_owned(),
    };
    let port = match endpoint.port() {
        ProfileEndpointPart::Literal(value) => value.as_str().to_owned(),
        ProfileEndpointPart::SecretSource => "•".to_owned(),
    };
    // Default database is not on list projection yet; host:port is the Phase-3 baseline.
    let target_summary = format!("{host}:{port}");
    let (environment, production_warning) = match item.environment() {
        Some(EnvironmentTag::Production) => (Some("production".into()), true),
        Some(EnvironmentTag::Staging) => (Some("staging".into()), false),
        Some(EnvironmentTag::Development) => (Some("development".into()), false),
        Some(EnvironmentTag::Testing) => (Some("testing".into()), false),
        Some(EnvironmentTag::Custom(label)) => (Some(label.as_str().to_owned()), false),
        None => (None, false),
    };
    ProfileRowProjection {
        id_hex: format!("{:032x}", id_as_u128(item.id())),
        name: item.name().as_str().to_owned(),
        engine_label: engine_label(item.engine()).to_owned(),
        group: item.group().map(|group| group.as_str().to_owned()),
        favorite: item.favorite(),
        target_summary,
        environment,
        production_warning,
        safety_label: safety_label(item.safety_mode()).to_owned(),
        plaintext_secret_warning: item.sources().has_dangerous_plaintext(),
        live_state: LiveConnectionState::Disconnected,
    }
}

const fn engine_label(engine: Engine) -> &'static str {
    match engine {
        Engine::PostgreSql => "PostgreSQL",
        Engine::ClickHouse => "ClickHouse",
        Engine::Redis => "Redis",
    }
}

const fn safety_label(mode: ProfileSafetyMode) -> &'static str {
    match mode {
        ProfileSafetyMode::ReadOnly => "Read only",
        ProfileSafetyMode::ConfirmWrites => "Confirm writes",
    }
}

fn id_as_u128(id: tablerock_core::ProfileId) -> u128 {
    let bytes = id.to_bytes();
    let mut high = [0_u8; 8];
    let mut low = [0_u8; 8];
    high.copy_from_slice(&bytes[0..8]);
    low.copy_from_slice(&bytes[8..16]);
    (u64::from_be_bytes(high) as u128) << 64 | u64::from_be_bytes(low) as u128
}
