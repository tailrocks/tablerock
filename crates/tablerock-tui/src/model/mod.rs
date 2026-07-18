//! Root-owned terminal presentation state.

pub mod catalog;
pub mod completion;
pub mod copy_format;
pub mod editor;
pub mod grid;
pub mod history;
pub mod inspector;
pub mod structure_ddl;
pub mod mutation_draft;
pub mod mutation_plan_build;
pub mod redis_command;
pub mod redis_key_view;
pub mod redis_namespace;
pub mod result_sections;
pub mod redis_stage;
pub mod saved_filter;
pub mod vim_mode;
pub mod profiles;
pub mod query_editor;
pub mod saved_query;
pub mod workbench;

use termrock::{
    Theme,
    input::{KeyCode, KeyEvent, KeyModifiers},
    interaction::{FocusOutcome, FocusRing},
    keymap::Keymap,
};

use crate::{ShellGeometry, ShellKeyAction, default_keymap, effect::RequestToken};
use editor::ConnectionFormModel;
use profiles::ProfileListState;
use workbench::WorkbenchModel;

pub const MINIMUM_WIDTH: u16 = 40;
pub const MINIMUM_HEIGHT: u16 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Wide,
    Medium,
    Narrow,
    TooSmall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusRegion {
    Context,
    Catalog,
    Tabs,
    Content,
    Actions,
    Footer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusScope {
    Shell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionId {
    Open,
    New,
    Save,
    Test,
    Connect,
    Disconnect,
    Remove,
    /// Cycle database selector (workbench).
    NextDatabase,
    NextTab,
    /// Select previous workbench tab (wrap).
    PrevTab,
    CloseTab,
    /// Close all tabs except the active one (fails closed if others dirty).
    CloseOtherTabs,
    /// Rename the active workbench tab title.
    RenameTab,
    PinTab,
    NewSql,
    RunSql,
    /// Run entire editor buffer as multi-statement script (ordered sections).
    RunScript,
    CancelQuery,
    Inspect,
    /// Close the inspector panel if open.
    CloseInspector,
    /// Open or commit SQL completion for the active editor.
    Complete,
    /// Open/refresh query history panel.
    History,
    /// Restore selected history entry into the active SQL editor.
    RestoreHistory,
    /// Open/refresh named saved queries panel.
    SavedQueries,
    /// Save active editor text as a named query (uses tab title as name).
    SaveQuery,
    /// Load selected named query into the editor.
    LoadQuery,
    /// Save active editor to its bound `.sql` path (or prompt path later).
    SaveFile,
    /// Remember intent-only session for the connected profile.
    SaveIntent,
    /// Copy loaded result as CSV (clipboard via OSC 52).
    CopyCsv,
    /// Copy loaded result as TSV.
    CopyTsv,
    CopyJson,
    CopyMarkdown,
    CopySqlInsert,
    CopySqlUpdate,
    /// Copy cursor cell raw text to clipboard.
    CopyCell,
    /// Copy cursor cell as hex of UTF-8 bytes.
    CopyCellHex,
    /// Copy cursor row (visible columns) as TSV.
    CopyRow,
    /// Copy cursor row as CSV.
    CopyRowCsv,
    /// Copy cursor row as JSON object array.
    CopyRowJson,
    /// Copy cursor row as Markdown table.
    CopyRowMarkdown,
    /// Copy cursor row as SQL INSERT (needs base-table identity).
    CopyRowSqlInsert,
    /// Copy cursor row as SQL UPDATE (needs base-table identity).
    CopyRowSqlUpdate,
    /// Format picker: paste `scope format` (row|loaded + csv|tsv|json|md|insert|update).
    CopyPick,
    /// Copy visible column names (tab-separated) to clipboard.
    CopyColumnNames,
    /// Copy cursor column values for all resident rows (one per line).
    CopyColumn,
    /// Cycle sort on the cursor column and re-browse when base table known.
    CycleSort,
    /// Append cursor column as secondary sort (or cycle its direction in place).
    PushSort,
    /// Remove least-significant (last) sort key and re-browse.
    PopSort,
    /// Clear server sort only (keep filters) and re-browse.
    ClearSort,
    /// Add an equality filter for the cursor column using the cursor cell text.
    AddFilter,
    /// Add IS NULL filter for the cursor column.
    FilterIsNull,
    /// Add IS NOT NULL filter for the cursor column.
    FilterIsNotNull,
    /// Add equality filter for empty string (column = '').
    FilterEmpty,
    /// Add inequality filter for empty string (column <> '').
    FilterNotEmpty,
    /// Remove the last server filter chip and re-browse.
    RemoveLastFilter,
    /// Remove all server filters for the cursor column and re-browse.
    RemoveColumnFilters,
    /// Add LIKE filter for cursor column using cell text with % wildcards.
    FilterLike,
    /// Add ILIKE filter (case-insensitive) for cursor column.
    FilterILike,
    /// Add inequality filter (ne) for cursor column using cell text.
    FilterNe,
    /// Comparison filters on cursor column (value from cell).
    FilterLt,
    FilterLe,
    FilterGt,
    FilterGe,
    /// Persist current grid filters as the named "default" preset for base table.
    SaveFilter,
    /// Apply the named "default" filter preset for the active base table.
    ApplyFilter,
    /// Clear server filters/sort and re-browse.
    ClearFilters,
    /// Paste/edit raw WHERE predicate for browse plan.
    EditRawWhere,
    /// Clear raw WHERE only (keep typed filter chips) and re-browse.
    ClearRawWhere,
    /// Copy filter + sort chip bar text to clipboard.
    CopyFilterBar,
    /// Edit page-local quick filter (no server I/O).
    EditQuickFilter,
    /// Clear page-local quick filter (no server I/O).
    ClearQuickFilter,
    /// Jump cursor to absolute row number (paste digit).
    GoToRow,
    /// Jump to first row (0).
    GoToFirstRow,
    /// Jump to last known row (Exact/Estimated totals).
    GoToLastRow,
    /// Jump to column by name (exact or unique prefix).
    GoToColumn,
    /// Re-browse the active base table (keep sort/filters).
    RefreshTable,
    /// Toggle visibility of the cursor column.
    ToggleColumn,
    /// Reset column layout to defaults.
    ResetColumns,
    /// Hide all columns except the cursor column.
    SoloColumn,
    /// Show all columns; keep widths/order (unlike ResetColumns).
    ShowAllColumns,
    /// Invert column visibility (at least one remains visible).
    InvertColumns,
    /// Persist column layout for the current base table.
    SaveColumns,
    /// Move cursor column left in display order.
    MoveColumnLeft,
    /// Move cursor column right in display order.
    MoveColumnRight,
    /// Narrow cursor column width by 2 (min 4).
    NarrowColumn,
    /// Widen cursor column width by 2 (max 64).
    WidenColumn,
    /// Fit cursor column width to resident content.
    FitColumn,
    /// Fit all visible columns to resident content.
    FitAllColumns,
    /// Undo last staged mutation draft action.
    UndoStaged,
    /// Discard all staged mutation drafts on the active tab.
    DiscardStaged,
    /// Open review dialog for staged mutations (typed plan preview).
    ReviewMutations,
    /// Begin inline edit of the cursor cell (editable results only).
    EditCell,
    /// Toggle boolean cell buffer while editing (type-specific).
    ToggleBool,
    /// Set cell edit buffer to null while editing.
    SetNull,
    /// Stamp today (YYYY-MM-DD) while editing a temporal cell.
    SetToday,
    /// Stamp now (YYYY-MM-DDTHH:MM:SSZ) while editing a temporal cell.
    SetNow,
    /// Step temporal date by +1 day while editing.
    IncDay,
    /// Step temporal date by -1 day while editing.
    DecDay,
    /// Step temporal date by +1 month while editing.
    IncMonth,
    /// Step temporal date by -1 month while editing.
    DecMonth,
    /// Open text month calendar for temporal edit (paste day 1-31).
    PickDate,
    /// Step number cell buffer by +1 while editing.
    IncNumber,
    /// Step number cell buffer by -1 while editing.
    DecNumber,
    /// Pretty-format structured/JSON cell buffer while editing.
    FormatJson,
    /// Compact structured/JSON cell buffer to one line while editing.
    CompactJson,
    /// Stage delete of the cursor row.
    DeleteRow,
    /// Stage a blank insert row (all columns empty/NULL).
    InsertRow,
    /// Stage an insert prefilled from the cursor row values.
    DuplicateRow,
    /// Edit values of the last staged insert (`col=value` lines).
    EditInsert,
    /// Discard only the last staged insert draft.
    DiscardLastInsert,
    /// Unstage cursor cell edit only (per-change discard).
    UnstageCell,
    /// Unstage all drafts for the cursor row (edits + delete).
    UnstageRow,
    /// Open inspector listing all staged drafts for the active grid.
    ShowStaged,
    /// Copy staged draft inventory text to clipboard.
    CopyStaged,
    /// Open inspector with bounded NOTICE history for the active grid tab.
    ShowNotices,
    /// Clear NOTICE history for the active grid tab.
    ClearNotices,
    /// Copy NOTICE history text to clipboard (OSC 52).
    CopyNotices,
    /// Page hex dump forward one 256-byte window.
    HexMore,
    /// Page hex dump backward one 256-byte window.
    HexLess,
    /// Expand structured JSON tree depth by one.
    ExpandTree,
    /// Collapse structured JSON tree depth by one.
    CollapseTree,
    /// Apply reviewed/staged mutations (typed plan rebuild from drafts).
    ApplyMutations,
    /// Follow FK from cursor column → filtered browse of referenced table.
    FollowForeignKey,
    /// Load structure facts (columns/types) into the inspector.
    ShowStructure,
    /// Copy CREATE TABLE DDL reconstructed from structure inspector (OSC 52).
    CopyStructureDdl,
    /// Request truncate of the active base table (gated confirm).
    TruncateTable,
    /// Request drop of the active base table (gated confirm).
    DropTable,
    /// VACUUM active base table (gated: re-type table name).
    VacuumTable,
    /// ANALYZE active base table (gated: re-type table name).
    AnalyzeTable,
    /// ClickHouse OPTIMIZE TABLE (gated: re-type table name; schema = database).
    OptimizeTable,
    /// Snapshot pg_stat_activity into the inspector.
    ShowActivity,
    /// Rename selected connection group (Connections tree).
    RenameGroup,
    /// Bounded automatic reconnect using current editor draft.
    Reconnect,
    /// Probe live session health (may auto-reconnect when preference allows).
    SessionHealth,
    /// Redis Pub/Sub: subscribe to a channel (isolated connection).
    RedisSubscribe,
    /// Redis Pub/Sub: pattern subscribe (PSUBSCRIBE).
    RedisPSubscribe,
    /// Snapshot roles + effective membership into the inspector.
    ShowRoles,
    /// Cancel a backend by pid (gated confirm).
    CancelBackend,
    /// Terminate a backend by pid (gated confirm).
    TerminateBackend,
    /// Kill a ClickHouse async mutation by id (gated re-type confirm).
    KillMutation,
    /// Rename active base table (gated: paste new name).
    RenameTable,
    /// Review ADD COLUMN: confirm_buffer = "column_name type".
    DdlAddColumn,
    /// Review CREATE INDEX: confirm_buffer = "index_name column".
    DdlCreateIndex,
    /// Review DROP COLUMN: confirm_buffer = column name.
    DdlDropColumn,
    /// Review DROP INDEX: confirm_buffer = index name.
    DdlDropIndex,
    /// Review ADD CONSTRAINT: confirm_buffer = "name UNIQUE (col)".
    DdlAddConstraint,
    /// Review DROP CONSTRAINT: confirm_buffer = constraint name.
    DdlDropConstraint,
    /// SCAN Redis keys in the connected logical DB.
    ScanRedisKeys,
    /// Load Redis INFO overview into the inspector.
    RedisInfo,
    /// Stage a Redis collection add (HSET/SADD/ZADD) for the open key view.
    StageRedisAdd,
    /// Stage a Redis collection remove (HDEL/SREM/ZREM) for the open key view.
    StageRedisRemove,
    /// Load next page of hash/set/zset collection entries for the open key.
    RedisCollectionMore,
    /// Export loaded result as CSV (path via paste/status; default export.csv).
    ExportCsv,
    ExportJson,
    ExportTsv,
    /// Streaming full re-query export (re-runs editor SQL to file).
    ExportStreamCsv,
    ExportStreamJson,
    ExportStreamTsv,
    /// Import CSV into active base table via mutation write seam (default import.csv).
    ImportCsv,
    /// Supervised pg_dump of the active connection endpoint (PostgreSQL only).
    PgDump,
    /// Supervised pg_restore into the active connection endpoint (PostgreSQL only).
    PgRestore,
    /// Paste a connection URL into the editor (reviewable draft).
    ImportUrl,
    /// Open an external/deep-link URL as a temporary session after confirm.
    OpenExternalUrl,
    /// Run EXPLAIN on the active SQL editor statement (PG/CH only).
    Explain,
    /// Fuzzy switch across open tabs (stable titles).
    QuickSwitch,
    /// Find/replace in the active SQL editor (literal; optional case-insensitive).
    FindReplace,
    /// Format the active SQL editor buffer (keyword case + whitespace).
    FormatSql,
    Submit,
    Cancel,
    Quit,
}

/// Pending destructive confirm (remove profile/group/tab / table ops).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmDialog {
    RemoveProfile { id_hex: String, name: String },
    RemoveGroup { name: String },
    /// Rename group: paste new name (non-empty, safe charset).
    RenameGroup {
        old_name: String,
        confirm_buffer: String,
    },
    CloseDirtyTab { title: String, index: usize },
    /// Require exact table name re-type for truncate (fail closed).
    TruncateTable {
        schema: String,
        table: String,
        confirm_buffer: String,
    },
    /// Require exact table name re-type for drop (fail closed).
    DropTable {
        schema: String,
        table: String,
        confirm_buffer: String,
    },
    /// Require exact table name re-type for VACUUM (fail closed).
    VacuumTable {
        schema: String,
        table: String,
        confirm_buffer: String,
    },
    /// Require exact table name re-type for ANALYZE (fail closed).
    AnalyzeTable {
        schema: String,
        table: String,
        confirm_buffer: String,
    },
    /// Require exact table name re-type for ClickHouse OPTIMIZE (fail closed).
    OptimizeTable {
        schema: String,
        table: String,
        confirm_buffer: String,
    },
    /// Cancel/terminate backend: confirm_buffer must equal pid digits.
    CancelBackend {
        pid: String,
        confirm_buffer: String,
    },
    TerminateBackend {
        pid: String,
        confirm_buffer: String,
    },
    /// Redis Pub/Sub: confirm_buffer is channel (or pattern when `pattern`).
    RedisSubscribe {
        pattern: bool,
        confirm_buffer: String,
    },
    /// Kill ClickHouse mutation: paste exact mutation_id for database.table.
    KillMutation {
        database: String,
        table: String,
        confirm_buffer: String,
    },
    /// Save filter preset: confirm_buffer is the preset name (non-empty).
    SaveFilter {
        schema: String,
        table: String,
        confirm_buffer: String,
    },
    /// Apply filter preset: confirm_buffer is the preset name to load.
    ApplyFilter {
        schema: String,
        table: String,
        /// Known names for the table (display only).
        known_names: Vec<String>,
        confirm_buffer: String,
    },
    /// Edit raw WHERE fragment for browse plan (paste SQL predicate only).
    EditRawWhere {
        confirm_buffer: String,
    },
    /// Edit page-local quick filter (resident rows only; no server I/O).
    EditQuickFilter {
        confirm_buffer: String,
    },
    /// Jump to absolute row: confirm_buffer is decimal row index.
    GoToRow {
        confirm_buffer: String,
    },
    /// Jump to column by name (case-sensitive exact or unique prefix).
    GoToColumn {
        confirm_buffer: String,
    },
    /// Rename active tab: confirm_buffer is the new title.
    RenameTab {
        confirm_buffer: String,
    },
    /// Text month calendar for temporal edit: paste day 1-31 (or full YYYY-MM-DD).
    PickDate {
        year: i32,
        month: u32,
        /// Day number or full ISO date.
        confirm_buffer: String,
        /// Time suffix to preserve (e.g. `T12:00:00Z`).
        time_suffix: String,
        /// Calendar grid for the dialog body.
        calendar_text: String,
    },
    /// Copy format picker: confirm_buffer is `scope format` (e.g. `row csv`, `loaded tsv`).
    CopyPick {
        confirm_buffer: String,
    },
    /// Edit last staged insert: confirm_buffer is `col=value` lines (empty value → NULL).
    EditInsertValues {
        draft_id: u64,
        confirm_buffer: String,
    },
    /// Stage Redis collection mutation: paste field/member/score payload.
    ///
    /// - hset: `field=value`
    /// - zadd: `score=member`
    /// - hdel/sadd/srem/zrem: bare field or member token
    StageRedis {
        /// hset | hdel | sadd | srem | zadd | zrem
        op: String,
        logical_db: String,
        key: String,
        confirm_buffer: String,
    },
    /// Rename: confirm_buffer is the new table name (non-empty, quoted later).
    RenameTable {
        schema: String,
        table: String,
        confirm_buffer: String,
    },
    /// Typed DDL review: preview shows plan label; buffer supplies object+type text.
    ///
    /// - add_column: buffer `"col type"`
    /// - create_index: buffer `"index_name column"`
    DdlReview {
        kind: String,
        schema: String,
        table: String,
        preview: String,
        confirm_buffer: String,
    },
    /// Authorize Write/Dangerous startup actions skipped at connect.
    ///
    /// Each item is `(safety_label, statement)`. Confirm buffer must equal `RUN`
    /// (case-sensitive) to authorize execution.
    StartupReview {
        items: Vec<(String, String)>,
        confirm_buffer: String,
    },
    /// pg_dump/pg_restore: paste destination (dump) or source (restore) path;
    /// empty buffer uses default `tablerock.dump`.
    PgTool {
        /// "dump" | "restore"
        kind: String,
        confirm_buffer: String,
    },
    /// Import connection URL: paste URL into buffer, Submit applies to editor.
    ImportUrl {
        confirm_buffer: String,
    },
    /// External URL open: paste URL, then paste OPEN to confirm connect.
    ///
    /// `summary` is redacted (no password text). `url` retained only for re-parse.
    /// When `matched_profile_id_hex` is set, OPEN uses saved profile (not temporary).
    OpenExternalUrl {
        /// Raw URL (not shown in status by default; used on confirm).
        url: String,
        summary: String,
        /// Saved profile id when engine+host:port/db matches a list row.
        matched_profile_id_hex: Option<String>,
        confirm_buffer: String,
    },
    /// Quick switch: paste filter or 1-based index, Submit selects matching tab.
    QuickSwitch {
        confirm_buffer: String,
    },
    /// Bind named SQL parameters: paste `name=value;…` then Submit runs.
    BindParams {
        names: Vec<String>,
        /// Original statement text (with `:name`); rewritten on submit.
        statement: String,
        confirm_buffer: String,
    },
    /// Find/replace: paste `find=>replace` or `find=>replace=>all` / `=>i` for
    /// case-insensitive; Submit applies.
    FindReplace {
        confirm_buffer: String,
    },
}

/// Ephemeral password prompt; Debug redacts buffer. Cleared after submit.
#[derive(Clone, PartialEq, Eq)]
pub struct PasswordPrompt {
    pub request_token: RequestToken,
    pub profile_id_hex: String,
    pub buffer: String,
}

impl std::fmt::Debug for PasswordPrompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasswordPrompt")
            .field("request_token", &self.request_token)
            .field("profile_id_hex", &self.profile_id_hex)
            .field("buffer_bytes", &self.buffer.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Connections,
    ConnectionPicker,
    /// First-version connection editor (new/edit).
    Editor,
    /// Stub workbench after Connect (session facts only until plan 007).
    Workbench,
}

/// Live session facts projected into the stub workbench.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionFacts {
    pub session_id_hex: String,
    pub identity: String,
    pub temporary: bool,
    pub engine_label: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellTarget {
    Focus(FocusRegion),
    Action(ActionId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl FocusRegion {
    pub(crate) const CONNECTION_ORDER: [Self; 6] = [
        Self::Context,
        Self::Catalog,
        Self::Tabs,
        Self::Content,
        Self::Actions,
        Self::Footer,
    ];

    pub(crate) const PICKER_ORDER: [Self; 5] = [
        Self::Context,
        Self::Tabs,
        Self::Content,
        Self::Actions,
        Self::Footer,
    ];

    pub(crate) const EDITOR_ORDER: [Self; 4] =
        [Self::Context, Self::Content, Self::Actions, Self::Footer];

    pub(crate) const WORKBENCH_ORDER: [Self; 4] =
        [Self::Context, Self::Content, Self::Actions, Self::Footer];
}

#[derive(Debug)]
pub struct Model {
    pub(crate) theme: Theme,
    keymap: Keymap<ShellKeyAction>,
    width: u16,
    height: u16,
    focus: FocusRing<FocusRegion, FocusScope>,
    action: ActionId,
    screen: Screen,
    terminal_focused: bool,
    hovered: Option<ShellTarget>,
    pressed: Option<ShellTarget>,
    engine_resync_required: bool,
    /// Monotonic effect correlation counter (no clocks).
    next_request_token: RequestToken,
    profiles: ProfileListState,
    editor: ConnectionFormModel,
    session: Option<SessionFacts>,
    workbench: WorkbenchModel,
    password_prompt: Option<PasswordPrompt>,
    confirm: Option<ConfirmDialog>,
    bootstrapped: bool,
    /// Last connect draft for reconnect (no secrets logged; may hold ephemeral password).
    pub(crate) last_connect_draft: Option<crate::effect::ConnectionDraft>,
    /// Reconnect preference label: "Manual" | "BoundedAutomatic".
    pub(crate) reconnect_preference: String,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            keymap: default_keymap(),
            width: 0,
            height: 0,
            focus: initial_focus_ring(),
            action: ActionId::Open,
            screen: Screen::Connections,
            terminal_focused: true,
            hovered: None,
            pressed: None,
            engine_resync_required: false,
            next_request_token: 1,
            profiles: ProfileListState::default(),
            editor: ConnectionFormModel::default(),
            session: None,
            workbench: WorkbenchModel::default(),
            password_prompt: None,
            confirm: None,
            bootstrapped: false,
            last_connect_draft: None,
            reconnect_preference: "Manual".into(),
        }
    }
}

impl Model {
    #[must_use]
    pub const fn keymap(&self) -> &Keymap<ShellKeyAction> {
        &self.keymap
    }

    pub fn keymap_mut(&mut self) -> &mut Keymap<ShellKeyAction> {
        &mut self.keymap
    }

    #[must_use]
    pub const fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    #[must_use]
    pub const fn focus(&self) -> Option<FocusRegion> {
        self.focus.focused().copied()
    }

    #[must_use]
    pub const fn selected_action(&self) -> ActionId {
        self.action
    }

    #[must_use]
    pub const fn screen(&self) -> Screen {
        self.screen
    }

    #[must_use]
    pub const fn terminal_focused(&self) -> bool {
        self.terminal_focused
    }

    #[must_use]
    pub const fn hovered(&self) -> Option<ShellTarget> {
        self.hovered
    }

    #[must_use]
    pub const fn pressed(&self) -> Option<ShellTarget> {
        self.pressed
    }

    #[must_use]
    pub const fn engine_resync_required(&self) -> bool {
        self.engine_resync_required
    }

    #[must_use]
    pub const fn profiles(&self) -> &ProfileListState {
        &self.profiles
    }

    #[must_use]
    pub const fn bootstrapped(&self) -> bool {
        self.bootstrapped
    }

    #[must_use]
    pub const fn layout_mode(&self) -> LayoutMode {
        if self.width < MINIMUM_WIDTH || self.height < MINIMUM_HEIGHT {
            LayoutMode::TooSmall
        } else if self.width >= 100 {
            LayoutMode::Wide
        } else if self.width >= 64 {
            LayoutMode::Medium
        } else {
            LayoutMode::Narrow
        }
    }

    pub(crate) const fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    pub(crate) fn request_focus(&mut self, focus: FocusRegion) -> bool {
        matches!(
            self.focus.request_focus(focus),
            FocusOutcome::Changed { .. }
        )
    }

    pub(crate) fn move_focus(&mut self, reverse: bool) -> bool {
        let key = KeyEvent::new(
            if reverse {
                KeyCode::BackTab
            } else {
                KeyCode::Tab
            },
            if reverse {
                KeyModifiers::SHIFT
            } else {
                KeyModifiers::NONE
            },
        );
        matches!(self.focus.handle_key(key), FocusOutcome::Changed { .. })
    }

    pub(crate) fn reconcile_focus_frame(&mut self, geometry: &ShellGeometry) -> bool {
        self.focus.begin_frame();
        let order: &[FocusRegion] = match self.screen {
            Screen::Connections => &FocusRegion::CONNECTION_ORDER,
            Screen::ConnectionPicker => &FocusRegion::PICKER_ORDER,
            Screen::Editor => &FocusRegion::EDITOR_ORDER,
            Screen::Workbench => &FocusRegion::WORKBENCH_ORDER,
        };
        let enabled = self.layout_mode() != LayoutMode::TooSmall;
        self.focus.register_order(
            FocusScope::Shell,
            order
                .iter()
                .copied()
                .map(|id| (id, geometry.focus_area(id), enabled)),
        );
        matches!(self.focus.reconcile(), FocusOutcome::Changed { .. })
    }

    pub(crate) const fn set_action(&mut self, action: ActionId) {
        self.action = action;
    }

    pub(crate) const fn set_screen(&mut self, screen: Screen) {
        self.screen = screen;
    }

    pub(crate) const fn set_terminal_focused(&mut self, focused: bool) {
        self.terminal_focused = focused;
        if !focused {
            self.hovered = None;
            self.pressed = None;
        }
    }

    pub(crate) const fn set_hovered(&mut self, target: Option<ShellTarget>) {
        self.hovered = target;
    }

    pub(crate) const fn set_pressed(&mut self, target: Option<ShellTarget>) {
        self.pressed = target;
    }

    pub(crate) const fn set_engine_resync_required(&mut self, required: bool) {
        self.engine_resync_required = required;
    }

    pub(crate) fn mint_request_token(&mut self) -> RequestToken {
        let token = self.next_request_token;
        self.next_request_token = self.next_request_token.saturating_add(1);
        token
    }

    pub(crate) fn set_profiles(&mut self, state: ProfileListState) {
        self.profiles = state;
    }

    pub(crate) fn profiles_mut(&mut self) -> &mut ProfileListState {
        &mut self.profiles
    }

    #[must_use]
    pub const fn editor(&self) -> &ConnectionFormModel {
        &self.editor
    }

    pub(crate) fn editor_mut(&mut self) -> &mut ConnectionFormModel {
        &mut self.editor
    }

    pub(crate) fn reset_editor(&mut self) {
        self.editor = ConnectionFormModel::default();
    }

    #[must_use]
    pub const fn session(&self) -> Option<&SessionFacts> {
        self.session.as_ref()
    }

    pub(crate) fn set_session(&mut self, session: Option<SessionFacts>) {
        self.session = session;
    }

    #[must_use]
    pub const fn workbench(&self) -> &WorkbenchModel {
        &self.workbench
    }

    pub(crate) fn workbench_mut(&mut self) -> &mut WorkbenchModel {
        &mut self.workbench
    }

    pub(crate) fn set_workbench(&mut self, workbench: WorkbenchModel) {
        self.workbench = workbench;
    }

    #[must_use]
    pub const fn password_prompt(&self) -> Option<&PasswordPrompt> {
        self.password_prompt.as_ref()
    }

    pub(crate) fn set_password_prompt(&mut self, prompt: Option<PasswordPrompt>) {
        self.password_prompt = prompt;
    }

    pub(crate) fn password_prompt_mut(&mut self) -> Option<&mut PasswordPrompt> {
        self.password_prompt.as_mut()
    }

    #[must_use]
    pub fn confirm_mut(&mut self) -> Option<&mut ConfirmDialog> {
        self.confirm.as_mut()
    }

    pub const fn confirm(&self) -> Option<&ConfirmDialog> {
        self.confirm.as_ref()
    }

    pub(crate) fn set_confirm(&mut self, confirm: Option<ConfirmDialog>) {
        self.confirm = confirm;
    }

    pub(crate) const fn set_bootstrapped(&mut self, value: bool) {
        self.bootstrapped = value;
    }
}

fn initial_focus_ring() -> FocusRing<FocusRegion, FocusScope> {
    let mut focus = FocusRing::new(FocusScope::Shell, Some(FocusRegion::Context));
    focus.begin_frame();
    focus.register_order(
        FocusScope::Shell,
        FocusRegion::CONNECTION_ORDER.map(|id| (id, None, true)),
    );
    let _ = focus.reconcile();
    focus
}
