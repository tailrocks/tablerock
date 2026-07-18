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
    /// Password taken from host environment variable at connect time.
    /// `var` is the variable name only — never the resolved value.
    HostEnvironment {
        var: String,
    },
    /// 1Password CLI reference (IDs only). Resolved via `op read` at connect.
    OnePassword {
        account_id: String,
        vault_id: String,
        item_id: String,
        section_id: Option<String>,
        field_id: String,
        breadcrumb: String,
    },
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
    /// Redis HSET field/value (hex or utf8 text; engine binds as bytes).
    RedisHashSet {
        field: String,
        value: String,
    },
    RedisHashDelete {
        field: String,
    },
    RedisSetAdd {
        member: String,
    },
    RedisSetRemove {
        member: String,
    },
    /// Score as decimal text; CLI parses to finite f64 bits.
    RedisZSetAdd {
        member: String,
        score: String,
    },
    RedisZSetRemove {
        member: String,
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
    /// Bastion host; empty means direct connect (no SSH tunnel).
    pub ssh_host: String,
    pub ssh_port: String,
    pub ssh_username: String,
    pub ssh_password: String,
    /// OpenSSH private key PEM (plaintext or encrypted) when using public-key auth.
    pub ssh_private_key: String,
    /// Absolute OpenSSH known_hosts path (required when `ssh_host` is set).
    pub ssh_known_hosts_path: String,
    /// When true, use SSH agent (`SSH_AUTH_SOCK`) instead of password/key material.
    pub ssh_use_agent: bool,
    /// Reviewed startup actions run after connect (ReadOnly auto-executes).
    pub startup_actions: tablerock_core::StartupActionSet,
    /// "Manual" or "BoundedAutomatic" (from profile preferences when saved).
    pub reconnect_preference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Exit,
    LoadProfileList {
        request_token: RequestToken,
        filter: ProfileListFilterSpec,
    },
    /// Probe registered session health (cheap server round-trip).
    CheckSessionHealth {
        request_token: RequestToken,
        session_id_hex: String,
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
    /// Rename a group (updates all members' group_name).
    RenameGroup {
        request_token: RequestToken,
        old_name: String,
        new_name: String,
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
    /// Build typed plan, review into the process registry (consume-once later).
    /// Preview lines come back on MutationReviewReady — never executed as SQL.
    ReviewMutations {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        database: String,
        schema: String,
        table: String,
        changes: Vec<MutationChangeSpec>,
    },
    /// Apply by review-token handle only (plan bytes never on this seam).
    ApplyMutations {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        /// 32-hex ReviewTokenId from MutationReviewReady.
        review_token_hex: String,
    },
    /// Load foreign-key edges for a base table (for FollowForeignKey).
    LoadForeignKeys {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        schema: String,
        table: String,
        /// Cursor column used to select which FK constraint to follow.
        local_column: String,
        /// All column→value pairs from the current row (multi-col FK support).
        row_cells: Vec<(String, String)>,
    },
    /// Load column structure facts into the inspector.
    LoadRelationStructure {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        schema: String,
        table: String,
    },
    /// Destructive table op after confirm (typed plan only — no free SQL).
    ExecuteTableOp {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        /// "truncate" | "drop" | "rename" | "vacuum" | "analyze" | "optimize"
        op: String,
        schema: String,
        table: String,
        /// New name for rename (empty otherwise).
        new_table: String,
    },
    /// Reviewed DDL plan execute (typed kind + identifiers — no free SQL).
    ExecuteDdlPlan {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        /// "add_column" | "create_index" | "drop_column" | ...
        kind: String,
        schema: String,
        table: String,
        object_name: String,
        /// Type / column list / constraint body when required by kind.
        type_text: String,
    },
    /// Snapshot activity for the inspector (permission-aware cancel later).
    LoadActivity {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
    },
    /// Load role list + effective membership (+ optional table grants) into inspector.
    LoadRoles {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        /// Optional relation for table privilege projection.
        schema: Option<String>,
        table: Option<String>,
    },
    /// Execute operator-authorized Write/Dangerous startup actions after review.
    ///
    /// Each item is `(safety_label, statement)` where safety is `write` or `danger`.
    ExecuteStartupReviewed {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        /// (safety, statement) pairs authorized by the confirm dialog.
        items: Vec<(String, String)>,
    },
    /// Supervised pg_dump (password only via env inside executor — never argv).
    RunPgDump {
        request_token: RequestToken,
        host: String,
        port: u16,
        database: String,
        username: String,
        /// Ephemeral; zeroed by executor after spawn.
        password: String,
        path: String,
        /// Optional absolute tool path; empty = PATH discovery.
        tool_path: String,
    },
    /// Supervised pg_restore (password only via env inside executor).
    RunPgRestore {
        request_token: RequestToken,
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
        path: String,
        tool_path: String,
    },
    /// pg_cancel_backend / pg_terminate_backend after pid confirm.
    SignalBackend {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        /// "cancel" | "terminate"
        kind: String,
        pid: i32,
    },
    /// ClickHouse KILL MUTATION after mutation_id re-type confirm.
    KillClickHouseMutation {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        database: String,
        table: String,
        mutation_id: String,
    },
    /// SCAN keys in the current Redis logical DB (never KEYS).
    ScanRedisKeys {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        /// SCAN MATCH pattern; empty means "*".
        pattern: String,
        /// Max keys to return in this page.
        count: u32,
    },
    /// Open a Redis key tab: TYPE + value projection for the key.
    OpenRedisKey {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        /// Key bytes as lossy UTF-8 or hex prefix (engine receives UTF-8 bytes).
        key: String,
        /// Collection entry skip for hash/set/zset next-page (0 = first page).
        collection_skip: u64,
    },
    /// Sequential Redis command pipeline (no MULTI/EXEC). Each line is argv text.
    ExecuteRedisPipeline {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        /// Pre-tokenized commands: (name, args as UTF-8 strings).
        commands: Vec<(String, Vec<String>)>,
    },
    /// Isolated BLPOP on a disposable connection (not the shared session multiplex).
    RedisBlockingPop {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        /// Key to BLPOP (first key only for this residual).
        key: String,
    },
    /// Redis Pub/Sub subscribe (isolated connection). Collects first page then completes.
    RedisSubscribe {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        /// Channel name or glob pattern.
        selector: String,
        /// true = PSUBSCRIBE, false = SUBSCRIBE.
        pattern: bool,
    },
    /// Load bounded INFO snapshot into the inspector.
    LoadRedisInfo {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
    },
    /// Export the loaded grid result to a file (CSV/JSON/TSV).
    ExportResult {
        request_token: RequestToken,
        /// Absolute or relative destination path.
        path: String,
        /// "csv" | "json" | "tsv"
        format: String,
        /// Preformatted body from pure formatters (no credentials).
        body: String,
    },
    /// Streaming full re-query export: re-run SQL and write pages atomically.
    ///
    /// Cancel via the shared cancel path for the session; incomplete files are removed.
    ExportStreamQuery {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        statement: String,
        path: String,
        /// "csv" | "json" | "tsv"
        format: String,
    },
    /// Import a CSV file into a relation through the typed mutation write seam.
    ImportCsvApply {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        /// Database name (ClickHouse) or PostgreSQL database label.
        database: String,
        schema: String,
        table: String,
        /// Destination path of the CSV to read (default import.csv).
        path: String,
    },
    /// Run a single SQL statement (first page) into the active tab grid.
    ExecuteSql {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        /// Already rewritten to `$n` when named params were used.
        statement: String,
        /// Positional bind texts (order matches `$n`); empty = no parameters.
        /// Executor maps via `parse_bind_text` — never concatenated into SQL.
        parameters: Vec<String>,
    },
    /// Run multiple statements in order; section summaries + last grid page.
    ExecuteSqlScript {
        request_token: RequestToken,
        session_id_hex: String,
        context_revision: u64,
        /// Statements already rewritten (no named placeholders).
        statements: Vec<String>,
        /// Shared positional binds applied to every statement that has `$n`.
        parameters: Vec<String>,
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
    /// Persist the full named-filter library JSON for a profile.
    SaveSavedFilterLibrary {
        request_token: RequestToken,
        profile_id_hex: String,
        library_json: String,
    },
    /// Load the named-filter library JSON for a profile.
    LoadSavedFilterLibrary {
        request_token: RequestToken,
        profile_id_hex: String,
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
