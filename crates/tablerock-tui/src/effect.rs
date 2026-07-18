//! Effects requested by the pure root reducer.
//!
//! Domain payloads stay presentation-local plain data so `tablerock-tui`
//! never depends on engine or persistence crates.

/// Correlation token minted by the reducer (monotonic counter, no clocks).
pub type RequestToken = u64;

/// Presentation-local profile list filter (engine maps into core filters).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfileListFilterSpec {
    pub engine: Option<EngineKind>,
    pub favorites_only: bool,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    PostgreSql,
    ClickHouse,
    Redis,
}

/// Opaque profile identity for effects (string form of core ProfileId).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRef {
    pub id_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Exit,
    LoadProfileList {
        request_token: RequestToken,
        filter: ProfileListFilterSpec,
    },
    CheckSessionHealth {
        request_token: RequestToken,
        profile: ProfileRef,
    },
}
