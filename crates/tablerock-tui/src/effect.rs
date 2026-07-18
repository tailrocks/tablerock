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
pub enum PasswordSourceSpec {
    PromptOnConnect,
    DangerousPlaintext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlsModeSpec {
    Off,
    VerifyCa,
    VerifyFull,
}

/// Catalog level request (executor maps to engine CatalogRequest).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogLevelSpec {
    Root,
    Schemas { database: String },
    Relations { database: String, schema: String },
    Objects { database: String },
}

/// Plain mutation change for ApplyMutations (presentation → CLI rebuilds typed plan).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationChangeSpec {
    Insert {
        values: Vec<(String, String)>,
    },
    Update {
        locator: Vec<(String, String)>,
        assignments: Vec<(String, String)>,
    },
    Delete {
        locator: Vec<(String, String)>,
    },
}

/// First-version connection editor payload for create/save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionDraft {
    pub engine: EngineKind,
    pub name: String,
    pub group: String,
    pub environment: String,
    pub host: String,
    pub port: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub password_source: PasswordSourceSpec,
    pub tls_mode: TlsModeSpec,
    pub plaintext_acknowledged: bool,
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
    SaveConnection {
        request_token: RequestToken,
        draft: ConnectionDraft,
    },
    /// Connect, describe server, disconnect — do not persist or register.
    TestConnection {
        request_token: RequestToken,
        draft: ConnectionDraft,
    },
    /// Connect, describe, register session; optional temporary (no save).
    ConnectSession {
        request_token: RequestToken,
        draft: ConnectionDraft,
        temporary: bool,
    },
    /// Shut down and remove a registered session.
    DisconnectSession {
        request_token: RequestToken,
        session_id_hex: String,
    },
    /// Load a saved profile and connect (non-temporary).
    ConnectProfile {
        request_token: RequestToken,
        profile_id_hex: String,
    },
    /// Resume profile connect after password prompt (secret lives only here).
    ResumeConnectProfile {
        request_token: RequestToken,
        profile_id_hex: String,
        /// Ephemeral password bytes; never log; executor zeros after use.
        password: String,
    },
    /// Bounded-backoff reconnect for a live session (draft re-connect path).
    ReconnectSession {
        request_token: RequestToken,
        draft: ConnectionDraft,
        attempt: u32,
    },
    /// Delete a saved profile (caller confirmed; active session check is UI-side).
    DeleteProfile {
        request_token: RequestToken,
        profile_id_hex: String,
    },
    /// Delete a group name (members become ungrouped).
    DeleteGroup {
        request_token: RequestToken,
        group_name: String,
    },
    /// Load one catalog level from a registered session.
    LoadCatalog {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        /// Engine label: PostgreSQL / ClickHouse / Redis.
        engine_label: String,
        level: CatalogLevelSpec,
        /// Parent presentation id for merge (None = roots).
        parent_id: Option<String>,
    },
    /// Browse a table via typed plan (sort/filter/raw WHERE) into the active grid.
    BrowseTable {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        schema: String,
        table: String,
        /// Sort keys as (column, "asc"|"desc").
        sort: Vec<(String, String)>,
        /// Filters as (column, operator, optional value).
        filters: Vec<(String, String, Option<String>)>,
        raw_where: Option<String>,
    },
    /// Apply staged mutations rebuilt as a typed plan (never from preview text).
    ApplyMutations {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        database: String,
        schema: String,
        table: String,
        /// Plain change specs: ("insert"|"update"|"delete", fields).
        /// Insert/update fields are (column, value_text); update/delete locators
        /// are separate as ("@locator", column, value_text) triples encoded in
        /// the payload builder.
        changes: Vec<MutationChangeSpec>,
    },
    /// Load foreign-key edges for a base table (for FollowForeignKey).
    LoadForeignKeys {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        schema: String,
        table: String,
        /// Column the operator is on; filter edges to this local column.
        local_column: String,
        /// Cell text used as the equality filter value on the foreign side.
        cell_value: String,
    },
    /// Load column structure facts into the inspector.
    LoadRelationStructure {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        schema: String,
        table: String,
    },
    /// Run a single SQL statement (first page) into the active tab grid.
    ExecuteSql {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        statement: String,
    },
    /// Cancel the active stream for a session (best-effort).
    CancelQuery {
        request_token: RequestToken,
        session_id_hex: String,
    },
    /// Project an already-admitted ResultStore page into the grid resident window.
    ///
    /// `result_token` is the original Execute/Browse request token used as the
    /// ResultId seed; pages were pumped into the store during that effect.
    FetchPage {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        result_token: RequestToken,
        start_row: u64,
    },
    /// List/search bounded query history (newest first).
    LoadHistory {
        request_token: RequestToken,
        search: Option<String>,
        limit: u32,
    },
    /// Append one executed statement to history (retention-aware).
    AppendHistory {
        request_token: RequestToken,
        engine_label: String,
        database: String,
        schema: Option<String>,
        statement: String,
        outcome: String,
        /// "full" | "metadata" | "private"
        retention: String,
    },
    /// Upsert a named saved query (statement text only).
    SaveNamedQuery {
        request_token: RequestToken,
        name: String,
        engine_label: String,
        statement: String,
    },
    /// List named saved queries for the active engine.
    ListNamedQueries {
        request_token: RequestToken,
        engine_label: String,
    },
    /// Load one saved query by id into the editor (caller restores text).
    LoadNamedQuery {
        request_token: RequestToken,
        query_id: i64,
    },
    /// Atomic write of editor text to a `.sql` path.
    SaveSqlFile {
        request_token: RequestToken,
        path: String,
        text: String,
    },
    /// Read a `.sql` file into the editor.
    OpenSqlFile {
        request_token: RequestToken,
        path: String,
    },
    /// Persist intent-only session (tabs/context text; never results).
    SaveSessionIntent {
        request_token: RequestToken,
        profile_id_hex: String,
        intent_json: String,
    },
    /// Load intent-only session for a profile.
    LoadSessionIntent {
        request_token: RequestToken,
        profile_id_hex: String,
    },
    /// Copy UTF-8 payload to the terminal clipboard (OSC 52).
    CopyToClipboard {
        request_token: RequestToken,
        text: String,
    },
    /// Persist per-table column layout JSON.
    SaveColumnLayout {
        request_token: RequestToken,
        profile_id_hex: String,
        database: String,
        schema: String,
        table: String,
        layout_json: String,
    },
    /// Load per-table column layout JSON.
    LoadColumnLayout {
        request_token: RequestToken,
        profile_id_hex: String,
        database: String,
        schema: String,
        table: String,
    },
}

/// Helper: build a root LoadCatalog effect for the current workbench session.
#[must_use]
pub fn load_root_catalog_effect(
    request_token: RequestToken,
    session_id_hex: String,
    context_revision: u64,
    engine_label: String,
) -> Effect {
    Effect::LoadCatalog {
        request_token,
        session_id_hex,
        context_revision,
        engine_label,
        level: CatalogLevelSpec::Root,
        parent_id: None,
    }
}
