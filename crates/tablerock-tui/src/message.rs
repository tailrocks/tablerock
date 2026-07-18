//! Facts and semantic intents accepted by the root reducer.

use std::fmt;

use crate::{ScrollDirection, ShellGeometry, ShellTarget};

pub const MAX_PASTE_BYTES: usize = 1_048_576;

#[derive(Clone, PartialEq, Eq)]
pub struct PasteText {
    text: String,
    truncated: bool,
}

impl PasteText {
    #[must_use]
    pub fn bounded(mut text: String) -> Self {
        let mut truncated = text.len() > MAX_PASTE_BYTES;
        if truncated {
            let mut boundary = MAX_PASTE_BYTES;
            while !text.is_char_boundary(boundary) {
                boundary -= 1;
            }
            text.truncate(boundary);
            truncated = true;
        }
        Self { text, truncated }
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub const fn was_truncated(&self) -> bool {
        self.truncated
    }
}

impl fmt::Debug for PasteText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PasteText")
            .field("bytes", &self.text.len())
            .field("truncated", &self.truncated)
            .finish()
    }
}

use crate::model::profiles::{FailureProjection, ProfileRowProjection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfilesMsg {
    ListLoaded {
        request_token: u64,
        items: Vec<ProfileRowProjection>,
    },
    ListFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    Saved {
        request_token: u64,
    },
    SaveFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    Deleted {
        request_token: u64,
    },
    DeleteFailed {
        request_token: u64,
        reason: FailureProjection,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineMsg {
    HealthOk {
        request_token: u64,
    },
    HealthFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    TestOk {
        request_token: u64,
        identity: String,
        elapsed_millis: u64,
        /// Optional startup-actions summary (ok/skip/fail counts).
        startup_summary: Option<String>,
    },
    TestFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    ConnectOk {
        request_token: u64,
        session_id_hex: String,
        identity: String,
        temporary: bool,
        engine_label: String,
        /// Non-temporary connects carry the profile for intent restore.
        profile_id_hex: Option<String>,
        /// Optional startup-actions summary after connect.
        startup_summary: Option<String>,
        /// Write/Dangerous actions skipped at connect; open StartupReview when non-empty.
        /// Each entry is `(safety_label, statement)`.
        startup_pending: Vec<(String, String)>,
        /// Profile reconnect preference label for health auto-reconnect.
        reconnect_preference: Option<String>,
    },
    ConnectFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    DisconnectOk {
        request_token: u64,
        session_id_hex: String,
    },
    DisconnectFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    /// Prompt-on-connect required; no network I/O happened yet.
    PasswordPromptRequired {
        request_token: u64,
        profile_id_hex: String,
    },
    Reconnecting {
        request_token: u64,
        attempt: u32,
        next_delay_ms: u64,
        /// Draft needed to re-dispatch the next attempt after delay (executor sleeps).
        draft: crate::effect::ConnectionDraft,
    },
    ReconnectStopped {
        request_token: u64,
        reason: FailureProjection,
    },
    CatalogLoaded {
        request_token: u64,
        context_revision: u64,
        parent_id: Option<String>,
        nodes: Vec<crate::model::catalog::CatalogNodeProjection>,
        truncated: bool,
    },
    CatalogFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
    },
    /// First page (or next page) projected for the grid.
    GridPage {
        request_token: u64,
        context_revision: u64,
        start_row: u64,
        columns: Vec<String>,
        cells: Vec<crate::model::grid::ProjectedCell>,
        row_count: u32,
        totals_exact: Option<u64>,
        totals_estimated: Option<u64>,
        bytes: u64,
        truncated: bool,
        complete: bool,
        /// When present (browse first page), proven primary-key column names.
        identity_columns: Option<Vec<String>>,
        /// Server/client query id (ClickHouse cancel target); shown while running.
        server_query_id: Option<String>,
        /// ClickHouse X-ClickHouse-Summary (or similar) progress label for status.
        server_progress: Option<String>,
    },
    GridFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
    },
    GridCancelDispatched {
        request_token: u64,
        /// Distinct dispatch fact: request_sent | prevented | transport_failed |
        /// server_rejected | unsupported | unknown
        dispatch: String,
    },
    GridCancelled {
        request_token: u64,
        label: String,
    },
    /// Stream pump finished after the first page was already delivered.
    GridStreamComplete {
        request_token: u64,
        context_revision: u64,
        rows_loaded: u64,
        truncated: bool,
        /// Bounded PostgreSQL NOTICE summary (severity + message); never secrets.
        notice_summary: Option<String>,
    },
    HistoryLoaded {
        request_token: u64,
        entries: Vec<crate::model::history::HistoryRowProjection>,
    },
    HistoryFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    HistoryAppended {
        request_token: u64,
        history_id: Option<i64>,
    },
    NamedQuerySaved {
        request_token: u64,
        query_id: i64,
        name: String,
    },
    NamedQueriesLoaded {
        request_token: u64,
        entries: Vec<crate::model::saved_query::SavedQueryRow>,
    },
    NamedQueryLoaded {
        request_token: u64,
        name: String,
        statement: String,
    },
    SqlFileSaved {
        request_token: u64,
        path: String,
        mtime_secs: Option<u64>,
        len: u64,
    },
    SqlFileOpened {
        request_token: u64,
        path: String,
        text: String,
        mtime_secs: Option<u64>,
        len: u64,
    },
    SqlFileFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    SessionIntentSaved {
        request_token: u64,
    },
    SessionIntentLoaded {
        request_token: u64,
        intent_json: Option<String>,
    },
    SessionIntentFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    ClipboardCopied {
        request_token: u64,
        bytes: usize,
    },
    ClipboardFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    ColumnLayoutLoaded {
        request_token: u64,
        layout_json: Option<String>,
    },
    ColumnLayoutSaved {
        request_token: u64,
    },
    ColumnLayoutFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    /// Named filter library JSON for the connected profile (None = empty).
    SavedFilterLibraryLoaded {
        request_token: u64,
        library_json: Option<String>,
    },
    SavedFilterLibrarySaved {
        request_token: u64,
    },
    SavedFilterLibraryFailed {
        request_token: u64,
        reason: FailureProjection,
    },
    /// Review registered; apply must use `review_token_hex` before expiry.
    MutationReviewReady {
        request_token: u64,
        context_revision: u64,
        review_token_hex: String,
        expires_at_ms: u64,
        /// Descriptive preview lines (SQL + param summaries); never executed.
        lines: Vec<String>,
    },
    MutationReviewFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
    },
    MutationApplied {
        request_token: u64,
        context_revision: u64,
        committed: bool,
        change_count: usize,
        detail: String,
    },
    MutationFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
        /// When true, operator must Review again (expired/consumed/missing token).
        needs_re_review: bool,
    },
    /// FK edge for follow-navigation: open filtered browse of foreign table.
    ///
    /// `filters` is ordered foreign_column=value pairs (multi-column FKs
    /// carry every key part from the source row).
    ForeignKeyEdge {
        request_token: u64,
        context_revision: u64,
        foreign_schema: String,
        foreign_table: String,
        /// (foreign_column, value) equality filters for the target browse.
        filters: Vec<(String, String)>,
    },
    ForeignKeysFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
    },
    /// Column structure lines for the inspector panel.
    RelationStructure {
        request_token: u64,
        context_revision: u64,
        schema: String,
        table: String,
        /// (name, type, not_null, default) display lines.
        columns: Vec<String>,
    },
    RelationStructureFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
    },
    TableOpDone {
        request_token: u64,
        context_revision: u64,
        op: String,
        schema: String,
        table: String,
    },
    TableOpFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
    },
    ActivitySnapshot {
        request_token: u64,
        context_revision: u64,
        lines: Vec<String>,
    },
    ActivityFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
    },
    /// Role list + effective membership lines for the inspector.
    RolesSnapshot {
        request_token: u64,
        context_revision: u64,
        lines: Vec<String>,
    },
    RolesFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
    },
    /// Outcome of authorized Write/Dangerous startup execution after review.
    StartupReviewDone {
        request_token: u64,
        summary: String,
    },
    /// pg_dump / pg_restore supervised run finished.
    PgToolDone {
        request_token: u64,
        /// "dump" | "restore"
        kind: String,
        summary: String,
        ok: bool,
    },
    /// Multi-statement script section snapshot (ordered; earlier never dropped).
    ScriptSections {
        request_token: u64,
        context_revision: u64,
        /// Display lines from ResultSectionsModel.
        lines: Vec<String>,
    },
    BackendSignalDone {
        request_token: u64,
        context_revision: u64,
        kind: String,
        pid: i32,
        acknowledged: bool,
    },
    BackendSignalFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
    },
    /// ClickHouse KILL MUTATION accepted; status_lines are system.mutations facts.
    MutationKillDone {
        request_token: u64,
        context_revision: u64,
        database: String,
        table: String,
        mutation_id: String,
        status_lines: Vec<String>,
    },
    MutationKillFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
    },
    /// SCAN page of Redis keys (display strings).
    RedisKeysLoaded {
        request_token: u64,
        context_revision: u64,
        keys: Vec<String>,
        /// True when more keys may exist (cursor not exhausted).
        has_more: bool,
    },
    RedisKeysFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
    },
    /// Type-specific Redis key view lines for inspector/grid header.
    RedisKeyViewLoaded {
        request_token: u64,
        context_revision: u64,
        key: String,
        kind_label: String,
        lines: Vec<String>,
        /// When set, more collection entries exist; open again with this skip.
        next_collection_skip: Option<u64>,
    },
    RedisKeyViewFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
    },
    RedisInfoLoaded {
        request_token: u64,
        context_revision: u64,
        sampled_at_ms: u64,
        lines: Vec<String>,
    },
    /// Sequential Redis pipeline outcomes (command summary + ok + detail).
    RedisPipelineDone {
        request_token: u64,
        context_revision: u64,
        /// Display lines for inspector / sections.
        lines: Vec<String>,
        ok_count: u32,
        fail_count: u32,
    },
    RedisPipelineFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
    },
    /// Incremental Pub/Sub batch while subscription is live (grid stays Running).
    RedisSubscribePage {
        request_token: u64,
        context_revision: u64,
        selector: String,
        pattern: bool,
        /// New message lines in this batch only.
        lines: Vec<String>,
        /// Total messages collected so far (including this batch).
        total_messages: u32,
    },
    /// Pub/Sub terminal: full collected lines (channel + payload) for inspector.
    RedisSubscribeDone {
        request_token: u64,
        context_revision: u64,
        selector: String,
        pattern: bool,
        lines: Vec<String>,
        /// True when first-page wait timed out with zero messages.
        timed_out: bool,
        /// True when pump stopped after idle gap before any listen-until-cancel phase.
        idle_stop: bool,
        /// True when operator Cancel stopped a live listen (not a failure).
        cancelled: bool,
    },
    RedisSubscribeFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
    },
    RedisInfoFailed {
        request_token: u64,
        context_revision: u64,
        reason: FailureProjection,
    },
    ExportDone {
        request_token: u64,
        path: String,
        bytes: u64,
    },
    ExportFailed {
        request_token: u64,
        reason: FailureProjection,
        /// True when a partial temp/destination was removed.
        partial_removed: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Resize {
        width: u16,
        height: u16,
    },
    FrameRendered(ShellGeometry),
    TerminalFocusChanged(bool),
    Paste(PasteText),
    PointerHovered(Option<ShellTarget>),
    PointerPressed(Option<ShellTarget>),
    PointerDragged(Option<ShellTarget>),
    PointerReleased(Option<ShellTarget>),
    PointerScrolled {
        target: Option<ShellTarget>,
        direction: ScrollDirection,
    },
    EngineResyncRequired,
    EngineResynchronized,
    Profiles(ProfilesMsg),
    Engine(EngineMsg),
    /// Periodic session health probe (CLI interval; BoundedAutomatic only).
    HealthTick,
    FocusNext,
    FocusPrevious,
    ActionNext,
    ActionPrevious,
    Activate,
    RequestRedraw,
    Quit,
}
