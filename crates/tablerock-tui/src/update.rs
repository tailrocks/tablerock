//! Deterministic root update path.

use crate::{
    ActionId, Effect, FocusRegion, Message, Model, Screen, ScrollDirection, ShellTarget,
    effect::ProfileListFilterSpec,
    message::{EngineMsg, ProfilesMsg},
    model::{
        ConfirmDialog, PasswordPrompt, SessionFacts,
        catalog::{CatalogModel, CatalogNodeStatus},
        grid::{GridOperationState, GridRowTotal},
        profiles::{FailureProjection, ProfileListState},
        workbench::WorkbenchModel,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    render: bool,
    effect: Option<Effect>,
}

impl Update {
    const fn unchanged() -> Self {
        Self {
            render: false,
            effect: None,
        }
    }

    const fn render() -> Self {
        Self {
            render: true,
            effect: None,
        }
    }

    const fn with_effect(effect: Effect) -> Self {
        Self {
            render: false,
            effect: Some(effect),
        }
    }

    #[must_use]
    pub const fn needs_render(&self) -> bool {
        self.render
    }

    pub fn effects(&self) -> impl Iterator<Item = &Effect> {
        self.effect.iter()
    }
}

pub fn update(model: &mut Model, message: Message) -> Update {
    match message {
        Message::Resize { width, height } => {
            let size_changed = model.size() != (width, height);
            if size_changed {
                model.resize(width, height);
            }
            let bootstrap = maybe_bootstrap_profiles(model);
            if bootstrap.effect.is_some() {
                return Update {
                    render: true,
                    effect: bootstrap.effect,
                };
            }
            if size_changed {
                Update::render()
            } else {
                Update::unchanged()
            }
        }
        Message::FrameRendered(geometry) => {
            if model.reconcile_focus_frame(&geometry) {
                Update::render()
            } else {
                Update::unchanged()
            }
        }
        Message::TerminalFocusChanged(focused) => {
            if model.terminal_focused() == focused {
                Update::unchanged()
            } else {
                model.set_terminal_focused(focused);
                Update::render()
            }
        }
        Message::Paste(text) if model.password_prompt().is_some() => {
            if let Some(prompt) = model.password_prompt_mut() {
                prompt.buffer.push_str(text.text());
            }
            Update::render()
        }
        Message::Paste(text) if model.confirm().is_some() => {
            if let Some(confirm) = model.confirm_mut() {
                match confirm {
                    ConfirmDialog::TruncateTable { confirm_buffer, .. }
                    | ConfirmDialog::DropTable { confirm_buffer, .. }
                    | ConfirmDialog::VacuumTable { confirm_buffer, .. }
                    | ConfirmDialog::AnalyzeTable { confirm_buffer, .. }
                    | ConfirmDialog::OptimizeTable { confirm_buffer, .. }
                    | ConfirmDialog::DdlReview { confirm_buffer, .. }
                    | ConfirmDialog::RenameTable { confirm_buffer, .. }
                    | ConfirmDialog::CancelBackend { confirm_buffer, .. }
                    | ConfirmDialog::TerminateBackend { confirm_buffer, .. }
                    | ConfirmDialog::KillMutation { confirm_buffer, .. }
                    | ConfirmDialog::SaveFilter { confirm_buffer, .. }
                    | ConfirmDialog::ApplyFilter { confirm_buffer, .. }
                    | ConfirmDialog::StageRedis { confirm_buffer, .. }
                    | ConfirmDialog::RedisSubscribe { confirm_buffer, .. }
                    | ConfirmDialog::RenameGroup { confirm_buffer, .. }
                    | ConfirmDialog::StartupReview { confirm_buffer, .. }
                    | ConfirmDialog::PgTool { confirm_buffer, .. }
                    | ConfirmDialog::ImportUrl { confirm_buffer, .. }
                    | ConfirmDialog::OpenExternalUrl { confirm_buffer, .. }
                    | ConfirmDialog::QuickSwitch { confirm_buffer, .. }
                    | ConfirmDialog::BindParams { confirm_buffer, .. }
                    | ConfirmDialog::FindReplace { confirm_buffer, .. } => {
                        *confirm_buffer = text.text().to_owned();
                    }
                    _ => {}
                }
            }
            Update::render()
        }
        Message::Paste(text) if model.screen() == Screen::Editor => {
            apply_editor_text(model, text.text());
            Update::render()
        }
        Message::Paste(text)
            if model.screen() == Screen::Connections
                && model.focus() == Some(FocusRegion::Content) =>
        {
            model.profiles_mut().push_search(text.text());
            Update::render()
        }
        Message::Paste(text)
            if model.screen() == Screen::Workbench
                && model.focus() == Some(FocusRegion::Catalog) =>
        {
            model.workbench_mut().catalog.push_filter(text.text());
            Update::render()
        }
        Message::Paste(text)
            if model.screen() == Screen::Workbench
                && model.focus() == Some(FocusRegion::Content) =>
        {
            // Inline cell edit takes paste before SQL editor.
            if let Some(grid) = model.workbench_mut().active_grid_mut()
                && let Some(edit) = grid.cell_edit.as_mut()
            {
                edit.buffer = text.text().to_owned();
                return Update::render();
            }
            let selected = model.workbench().selected_tab;
            if let Some(tab) = model.workbench_mut().tabs.get_mut(selected)
                && let Some(editor) = tab.editor.as_mut()
            {
                editor.insert(text.text());
                tab.dirty = true;
                return Update::render();
            }
            Update::unchanged()
        }
        Message::Paste(_) => Update::unchanged(),
        Message::PointerHovered(target) | Message::PointerDragged(target) => {
            if model.hovered() == target {
                Update::unchanged()
            } else {
                model.set_hovered(target);
                Update::render()
            }
        }
        Message::PointerPressed(target) => {
            model.set_pressed(target);
            if let Some(target) = target {
                focus_target(model, target);
            }
            Update::render()
        }
        Message::PointerReleased(target) => {
            let pressed = model.pressed();
            model.set_pressed(None);
            model.set_hovered(target);
            if pressed == target
                && let Some(ShellTarget::Action(action)) = target
            {
                model.set_action(action);
                return activate_selected_action(model);
            }
            Update::render()
        }
        Message::PointerScrolled { target, direction } => {
            if let Some(target) = target {
                focus_target(model, target);
            }
            if model.screen() == Screen::Workbench {
                return scroll_grid(model, direction);
            }
            if target.is_some() {
                Update::render()
            } else {
                Update::unchanged()
            }
        }
        Message::EngineResyncRequired => {
            if model.engine_resync_required() {
                Update::unchanged()
            } else {
                model.set_engine_resync_required(true);
                Update::render()
            }
        }
        Message::EngineResynchronized => {
            if model.engine_resync_required() {
                model.set_engine_resync_required(false);
                Update::render()
            } else {
                Update::unchanged()
            }
        }
        Message::Profiles(ProfilesMsg::ListLoaded {
            request_token,
            items,
        }) => {
            if model.profiles().active_token() != Some(request_token) {
                return Update::unchanged();
            }
            let selected_id = items.first().map(|row| row.id_hex.clone());
            model.set_profiles(ProfileListState::Loaded {
                request_token,
                rows: items,
                selected_id,
                search: String::new(),
                collapsed: Vec::new(),
            });
            Update::render()
        }
        Message::Profiles(ProfilesMsg::ListFailed {
            request_token,
            reason,
        }) => {
            if model.profiles().active_token() != Some(request_token) {
                return Update::unchanged();
            }
            model.set_profiles(ProfileListState::Failed {
                request_token,
                reason,
            });
            Update::render()
        }
        Message::Profiles(ProfilesMsg::Saved { request_token }) => {
            if model.profiles().active_token() != Some(request_token) {
                return Update::unchanged();
            }
            model.set_screen(Screen::Connections);
            model.set_action(ActionId::Open);
            // Reload list under a new token.
            let token = model.mint_request_token();
            model.set_profiles(ProfileListState::Loading {
                request_token: token,
            });
            Update {
                render: true,
                effect: Some(Effect::LoadProfileList {
                    request_token: token,
                    filter: ProfileListFilterSpec::default(),
                }),
            }
        }
        Message::Profiles(ProfilesMsg::SaveFailed {
            request_token,
            reason,
        }) => {
            if model.profiles().active_token() != Some(request_token) {
                return Update::unchanged();
            }
            model.editor_mut().validation_error = Some(match reason {
                FailureProjection::Label(label) => label,
            });
            Update::render()
        }
        Message::Profiles(ProfilesMsg::Deleted { request_token }) => {
            if model.profiles().active_token() != Some(request_token) {
                return Update::unchanged();
            }
            model.set_confirm(None);
            let token = model.mint_request_token();
            model.set_profiles(ProfileListState::Loading {
                request_token: token,
            });
            Update {
                render: true,
                effect: Some(Effect::LoadProfileList {
                    request_token: token,
                    filter: ProfileListFilterSpec::default(),
                }),
            }
        }
        Message::Profiles(ProfilesMsg::DeleteFailed {
            request_token,
            reason,
        }) => {
            if model.profiles().active_token() != Some(request_token) {
                return Update::unchanged();
            }
            model.set_confirm(None);
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            // Surface as list failure without dropping existing rows if possible.
            model.set_profiles(ProfileListState::Failed {
                request_token,
                reason: FailureProjection::Label(label),
            });
            Update::render()
        }
        Message::Engine(EngineMsg::HealthOk { .. }) => {
            if let Some(mut session) = model.session().cloned() {
                session.status = Some("healthy".into());
                model.set_session(Some(session));
            }
            model.workbench_mut().context.health_label = "healthy".into();
            Update::render()
        }
        Message::Engine(EngineMsg::HealthFailed { reason, .. }) => {
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(mut session) = model.session().cloned() {
                session.status = Some(format!("unhealthy: {label}"));
                model.set_session(Some(session));
            }
            model.workbench_mut().context.health_label = format!("unhealthy: {label}");
            // BoundedAutomatic: start reconnect attempt 0 from last draft.
            if crate::model::saved_filter::should_auto_reconnect(&model.reconnect_preference) {
                if let Some(draft) = model.last_connect_draft.clone() {
                    let token = model.mint_request_token();
                    return Update {
                        render: true,
                        effect: Some(Effect::ReconnectSession {
                            request_token: token,
                            draft,
                            attempt: 0,
                        }),
                    };
                }
            }
            Update::render()
        }
        Message::HealthTick => {
            // Continuous health only when a session is live and auto-reconnect is on.
            if model.session().is_none() {
                return Update::unchanged();
            }
            if !crate::model::saved_filter::should_auto_reconnect(&model.reconnect_preference) {
                return Update::unchanged();
            }
            // Skip while already reconnecting.
            if model
                .session()
                .and_then(|s| s.status.as_deref())
                .is_some_and(|s| s.contains("reconnecting"))
            {
                return Update::unchanged();
            }
            let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
                return Update::unchanged();
            };
            let token = model.mint_request_token();
            Update::with_effect(Effect::CheckSessionHealth {
                request_token: token,
                session_id_hex,
            })
        }
        Message::Engine(EngineMsg::TestOk {
            identity,
            elapsed_millis,
            startup_summary,
            ..
        }) => {
            let mut status = format!("ok: {identity} ({elapsed_millis} ms)");
            if let Some(summary) = startup_summary {
                status.push_str("; ");
                status.push_str(&summary);
            }
            model.editor_mut().test_status = Some(status);
            Update::render()
        }
        Message::Engine(EngineMsg::TestFailed { reason, .. }) => {
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            model.editor_mut().test_status = Some(format!("failed: {label}"));
            Update::render()
        }
        Message::Engine(EngineMsg::ConnectOk {
            session_id_hex,
            identity,
            temporary,
            engine_label,
            profile_id_hex,
            startup_summary,
            startup_pending,
            reconnect_preference,
            ..
        }) => {
            let status = match startup_summary {
                Some(summary) => format!("connected; {summary}"),
                None => "connected".into(),
            };
            if let Some(pref) = reconnect_preference {
                model.reconnect_preference = pref;
            }
            model.set_session(Some(SessionFacts {
                session_id_hex: session_id_hex.clone(),
                identity: identity.clone(),
                temporary,
                engine_label: engine_label.clone(),
                status: Some(status),
            }));
            let has_startup_review = !startup_pending.is_empty();
            if has_startup_review {
                model.set_confirm(Some(ConfirmDialog::StartupReview {
                    items: startup_pending,
                    confirm_buffer: String::new(),
                }));
            }
            let mut workbench = WorkbenchModel::from_session(
                if temporary { "temporary" } else { "profile" },
                engine_label.clone(),
                temporary,
                identity,
            );
            workbench.profile_id_hex = profile_id_hex.clone();
            let token = model.mint_request_token();
            let context_revision = workbench.context_revision;
            workbench.catalog = CatalogModel::Loading {
                request_token: token,
                context_revision,
            };
            model.set_workbench(workbench);
            model.set_screen(Screen::Workbench);
            // Startup review dialog takes the action bar (Submit/Cancel).
            if has_startup_review {
                model.set_action(ActionId::Submit);
            } else {
                model.set_action(ActionId::Disconnect);
            }
            // Prefer intent restore for non-temporary profiles; catalog still loads after.
            if let Some(profile_id_hex) = profile_id_hex.filter(|_| !temporary) {
                return Update {
                    render: true,
                    effect: Some(Effect::LoadSessionIntent {
                        request_token: token,
                        profile_id_hex,
                    }),
                };
            }
            Update {
                render: true,
                effect: Some(Effect::LoadCatalog {
                    request_token: token,
                    session_id_hex,
                    context_revision,
                    engine_label,
                    level: crate::effect::CatalogLevelSpec::Root,
                    parent_id: None,
                }),
            }
        }
        Message::Engine(EngineMsg::CatalogLoaded {
            request_token,
            context_revision,
            parent_id,
            nodes,
            truncated,
        }) => {
            let catalog = &model.workbench().catalog;
            if !catalog.accepts(request_token, context_revision) {
                return Update::unchanged();
            }
            // Promote Loading → Loaded on first root completion.
            if matches!(catalog, CatalogModel::Loading { .. }) && parent_id.is_none() {
                model.workbench_mut().catalog = CatalogModel::Loaded {
                    request_token,
                    context_revision,
                    nodes: Vec::new(),
                    selected_id: None,
                    filter: String::new(),
                    truncated: false,
                };
            }
            if matches!(model.workbench().catalog, CatalogModel::Loaded { .. }) {
                model.workbench_mut().catalog.merge_children(
                    parent_id.as_deref(),
                    nodes,
                    truncated,
                );
            }
            Update::render()
        }
        Message::Engine(EngineMsg::CatalogFailed {
            request_token,
            context_revision,
            reason,
        }) => {
            let catalog = &model.workbench().catalog;
            if !catalog.accepts(request_token, context_revision) {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            model.workbench_mut().catalog = CatalogModel::Failed {
                request_token,
                context_revision,
                reason: label,
            };
            Update::render()
        }
        Message::Engine(EngineMsg::GridPage {
            request_token,
            context_revision,
            start_row,
            columns,
            cells,
            row_count,
            totals_exact,
            totals_estimated,
            bytes,
            truncated,
            complete,
            identity_columns,
            server_query_id,
            server_progress,
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            // Disconnect mid-stream: keep stale page; never revive disconnected ops.
            if model
                .workbench()
                .active_grid()
                .is_some_and(|g| g.operation == GridOperationState::Disconnected)
            {
                return Update::unchanged();
            }
            let totals = if let Some(n) = totals_exact {
                GridRowTotal::Exact(n)
            } else if let Some(n) = totals_estimated {
                GridRowTotal::Estimated(n)
            } else {
                GridRowTotal::Unknown
            };
            let safety = safety_mode_from_label(&model.workbench().context.safety_label);
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                // First page from Execute/Browse stamps the result_token seed.
                if start_row == 0 {
                    grid.result_token = request_token;
                    if let Some(qid) = server_query_id {
                        grid.server_query_id = Some(qid);
                    }
                }
                if let Some(progress) = server_progress {
                    grid.server_progress = Some(progress);
                }
                if let Some(identity) = identity_columns {
                    grid.identity_columns = identity;
                    // Browse sets base_schema/table before the first page arrives.
                    // Ad-hoc SQL leaves base unset → stays read-only.
                    if grid.base_schema.is_some() && grid.base_table.is_some() {
                        grid.recompute_editability(safety, false);
                    }
                }
                // EXPLAIN result: open plan tree inspector from first-column lines.
                let explain_plan = if start_row == 0
                    && columns
                        .iter()
                        .any(|c| c.eq_ignore_ascii_case("QUERY PLAN") || c.eq_ignore_ascii_case("explain"))
                {
                    let col_count = columns.len().max(1);
                    let mut lines = Vec::new();
                    for (i, cell) in cells.iter().enumerate() {
                        if i % col_count == 0 {
                            lines.push(cell.text.clone());
                        }
                    }
                    Some(lines.join("\n"))
                } else {
                    None
                };
                grid.replace_page(
                    start_row, columns, cells, row_count, totals, bytes, truncated,
                );
                if complete {
                    grid.mark_completed();
                }
                if let Some(plan) = explain_plan {
                    model.workbench_mut().inspector =
                        crate::model::inspector::InspectorModel::from_explain_text(
                            "explain",
                            &plan,
                        );
                }
            }
            let selected = model.workbench().selected_tab;
            if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
                tab.running = !complete;
            }
            Update::render()
        }
        Message::Engine(EngineMsg::GridStreamComplete {
            request_token,
            context_revision,
            rows_loaded,
            truncated,
            notice_summary,
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            if model
                .workbench()
                .active_grid()
                .is_some_and(|g| g.operation == GridOperationState::Disconnected)
            {
                return Update::unchanged();
            }
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.rows_loaded = grid.rows_loaded.max(rows_loaded);
                if truncated {
                    grid.truncated = true;
                }
                grid.mark_completed();
                if let Some(summary) = notice_summary {
                    // Surface bounded NOTICE text in status (not SQL/values).
                    grid.error_label = Some(format!("notice: {summary}"));
                }
            }
            let selected = model.workbench().selected_tab;
            if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
                tab.running = false;
            }
            // Best-effort history append for the SQL that just completed.
            let statement = model
                .workbench()
                .active_editor()
                .and_then(|ed| ed.run_text())
                .unwrap_or_default();
            if statement.trim().is_empty() {
                return Update::render();
            }
            let engine_label = model.workbench().engine_kind.clone();
            let database = model.workbench().context.database.clone();
            let schema = model.workbench().context.schema.clone();
            let retention = model.workbench().history_retention.clone();
            Update {
                render: true,
                effect: Some(Effect::AppendHistory {
                    request_token,
                    engine_label,
                    database,
                    schema,
                    statement,
                    outcome: "completed".into(),
                    retention,
                }),
            }
        }
        Message::Engine(EngineMsg::HistoryLoaded {
            request_token,
            entries,
        }) => {
            let loading = matches!(
                model.workbench().history,
                crate::model::history::HistoryPanel::Loading { request_token: t }
                    if t == request_token
            );
            if !loading {
                return Update::unchanged();
            }
            model.workbench_mut().history = crate::model::history::HistoryPanel::Open {
                request_token,
                entries,
                selected: 0,
                search: String::new(),
            };
            Update::render()
        }
        Message::Engine(EngineMsg::HistoryFailed {
            request_token,
            reason,
        }) => {
            let loading = matches!(
                model.workbench().history,
                crate::model::history::HistoryPanel::Loading { request_token: t }
                    if t == request_token
            );
            if !loading {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            model.workbench_mut().history = crate::model::history::HistoryPanel::Failed {
                request_token,
                reason: label,
            };
            Update::render()
        }
        Message::Engine(EngineMsg::HistoryAppended { .. }) => Update::unchanged(),
        Message::Engine(EngineMsg::NamedQuerySaved { .. }) => Update::render(),
        Message::Engine(EngineMsg::NamedQueriesLoaded {
            request_token,
            entries,
        }) => {
            let ok = matches!(
                model.workbench().saved_queries,
                crate::model::saved_query::SavedQueryPanel::Loading { request_token: t }
                    if t == request_token
            );
            if !ok {
                return Update::unchanged();
            }
            model.workbench_mut().saved_queries =
                crate::model::saved_query::SavedQueryPanel::Open {
                    request_token,
                    entries,
                    selected: 0,
                };
            Update::render()
        }
        Message::Engine(EngineMsg::NamedQueryLoaded {
            name,
            statement,
            ..
        }) => {
            if model.workbench().active_editor().is_none() {
                model.workbench_mut().open_sql_tab();
            }
            if let Some(editor) = model.workbench_mut().active_editor_mut() {
                editor.set_text(statement);
            }
            let selected = model.workbench().selected_tab;
            if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
                tab.title = name;
                tab.dirty = false;
            }
            model.workbench_mut().saved_queries =
                crate::model::saved_query::SavedQueryPanel::Closed;
            Update::render()
        }
        Message::Engine(EngineMsg::SqlFileSaved {
            path,
            mtime_secs,
            len,
            ..
        }) => {
            let selected = model.workbench().selected_tab;
            if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
                tab.bound_file = Some(crate::model::saved_query::BoundSqlFile {
                    path,
                    mtime_secs,
                    len,
                });
                tab.dirty = false;
            }
            Update::render()
        }
        Message::Engine(EngineMsg::SqlFileOpened {
            path,
            text,
            mtime_secs,
            len,
            ..
        }) => {
            if model.workbench().active_editor().is_none() {
                model.workbench_mut().open_sql_tab();
            }
            if let Some(editor) = model.workbench_mut().active_editor_mut() {
                editor.set_text(text);
            }
            let selected = model.workbench().selected_tab;
            if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
                tab.bound_file = Some(crate::model::saved_query::BoundSqlFile {
                    path: path.clone(),
                    mtime_secs,
                    len,
                });
                tab.dirty = false;
                if let Some(name) = std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                {
                    tab.title = name.to_owned();
                }
            }
            Update::render()
        }
        Message::Engine(EngineMsg::SqlFileFailed { reason, .. }) => {
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::SessionIntentSaved { .. }) => Update::render(),
        Message::Engine(EngineMsg::SessionIntentLoaded {
            intent_json, ..
        }) => {
            if let Some(json) = intent_json {
                let _ = model.workbench_mut().apply_intent_json(&json);
            }
            // After intent, load saved filter library (then catalog).
            if let Some(profile_id_hex) = model.workbench().profile_id_hex.clone() {
                let token = model.mint_request_token();
                return Update {
                    render: true,
                    effect: Some(Effect::LoadSavedFilterLibrary {
                        request_token: token,
                        profile_id_hex,
                    }),
                };
            }
            load_workbench_root_catalog(model)
        }
        Message::Engine(EngineMsg::SessionIntentFailed { .. }) => {
            // Intent miss is non-fatal; still try filter library then catalog.
            if let Some(profile_id_hex) = model.workbench().profile_id_hex.clone() {
                let token = model.mint_request_token();
                return Update {
                    render: true,
                    effect: Some(Effect::LoadSavedFilterLibrary {
                        request_token: token,
                        profile_id_hex,
                    }),
                };
            }
            load_workbench_root_catalog(model)
        }
        Message::Engine(EngineMsg::ClipboardCopied { bytes, .. }) => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.error_label = Some(format!("copied {bytes} B"));
            }
            Update::render()
        }
        Message::Engine(EngineMsg::ClipboardFailed { reason, .. }) => {
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::ColumnLayoutLoaded { layout_json, .. }) => {
            if let Some(json) = layout_json {
                if let Some(grid) = model.workbench_mut().active_grid_mut() {
                    let _ = grid.apply_layout_json(&json);
                }
            }
            // After layout load (or miss), start the browse stream.
            rebrowse_active_table(model)
        }
        Message::Engine(EngineMsg::ColumnLayoutSaved { .. }) => Update::render(),
        Message::Engine(EngineMsg::ColumnLayoutFailed { reason, .. }) => {
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::SavedFilterLibraryLoaded { library_json, .. }) => {
            if let Some(json) = library_json {
                if let Some(lib) =
                    crate::model::saved_filter::SavedFilterLibrary::from_json(&json)
                {
                    model.workbench_mut().filter_library = lib;
                }
            }
            // Continue connect path: catalog for restored context.
            load_workbench_root_catalog(model)
        }
        Message::Engine(EngineMsg::SavedFilterLibrarySaved { .. }) => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.error_label = Some("filter preset saved".into());
            }
            Update::render()
        }
        Message::Engine(EngineMsg::SavedFilterLibraryFailed { reason, .. }) => {
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            // Connect-path load failures still open the catalog.
            if model.session().is_some()
                && matches!(model.workbench().catalog, CatalogModel::Idle)
            {
                return load_workbench_root_catalog(model);
            }
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::MutationReviewReady {
            context_revision,
            review_token_hex,
            expires_at_ms,
            lines,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            model.workbench_mut().pending_review_token_hex = Some(review_token_hex);
            model.workbench_mut().pending_review_expires_at_ms = Some(expires_at_ms);
            model.workbench_mut().mutation_review =
                Some(crate::model::mutation_plan_build::MutationReviewView {
                    mutation_id_hex: String::new(),
                    schema: model
                        .workbench()
                        .active_grid()
                        .and_then(|g| g.base_schema.clone())
                        .unwrap_or_default(),
                    table: model
                        .workbench()
                        .active_grid()
                        .and_then(|g| g.base_table.clone())
                        .unwrap_or_default(),
                    lines: lines
                        .into_iter()
                        .map(|sql| crate::model::mutation_plan_build::ReviewStatementLine {
                            sql,
                            parameters: Vec::new(),
                            kind: "review",
                        })
                        .collect(),
                });
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.error_label = Some("review ready — Apply before token expires".into());
            }
            Update::render()
        }
        Message::Engine(EngineMsg::MutationReviewFailed {
            context_revision,
            reason,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            model.workbench_mut().pending_review_token_hex = None;
            model.workbench_mut().pending_review_expires_at_ms = None;
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::MutationApplied {
            context_revision,
            committed,
            change_count,
            detail,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            model.workbench_mut().pending_review_token_hex = None;
            model.workbench_mut().pending_review_expires_at_ms = None;
            if committed {
                let was_redis = !model.workbench().redis_staged.is_empty();
                model.workbench_mut().redis_staged.clear();
                if let Some(grid) = model.workbench_mut().active_grid_mut() {
                    grid.drafts.discard_all();
                    grid.cell_edit = None;
                    grid.mark_completed();
                    grid.error_label = Some(format!("applied {change_count}: {detail}"));
                }
                model.workbench_mut().mutation_review = None;
                model.workbench_mut().mark_active_dirty(false);
                if was_redis {
                    // Re-open key view so collection page reflects server.
                    if let Some((_, key, _)) = model.workbench().redis_stage_target.clone() {
                        if let Some(session_id_hex) =
                            model.session().map(|s| s.session_id_hex.clone())
                        {
                            let token = model.mint_request_token();
                            let context_revision = model.workbench().context_revision;
                            return Update {
                                render: true,
                                effect: Some(Effect::OpenRedisKey {
                                    request_token: token,
                                    session_id_hex,
                                    context_revision,
                                    key,
                                    collection_skip: 0,
                                }),
                            };
                        }
                    }
                    return Update::render();
                }
                // Refresh base table so grid matches server.
                return rebrowse_active_table(model);
            }
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                // Conflict/rollback: keep staged drafts for resolution.
                grid.mark_failed(format!("apply rolled back: {detail}"));
            }
            Update::render()
        }
        Message::Engine(EngineMsg::MutationFailed {
            context_revision,
            reason,
            needs_re_review,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if needs_re_review {
                model.workbench_mut().pending_review_token_hex = None;
                model.workbench_mut().pending_review_expires_at_ms = None;
                model.workbench_mut().mutation_review = None;
            }
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::ForeignKeyEdge {
            context_revision,
            foreign_schema,
            foreign_table,
            filters,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
                return Update::unchanged();
            };
            if filters.is_empty() {
                return Update::unchanged();
            }
            let title = {
                let parts: Vec<_> = filters
                    .iter()
                    .map(|(c, v)| format!("{c}={v}"))
                    .collect();
                format!("{foreign_table} · {}", parts.join(","))
            };
            model.workbench_mut().open_preview_tab(&title);
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.base_schema = Some(foreign_schema.clone());
                grid.base_table = Some(foreign_table.clone());
                grid.clear_server_controls();
                for (foreign_column, filter_value) in filters {
                    grid.add_filter_chip(foreign_column, "eq", Some(filter_value));
                }
                grid.operation = GridOperationState::Running;
            }
            let token = model.mint_request_token();
            Update {
                render: true,
                effect: Some(browse_table_effect(
                    token,
                    session_id_hex,
                    context_revision,
                    foreign_schema,
                    foreign_table,
                    model.workbench().active_grid(),
                )),
            }
        }
        Message::Engine(EngineMsg::ForeignKeysFailed {
            context_revision,
            reason,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::RelationStructure {
            context_revision,
            schema,
            table,
            columns,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let mut body = columns.join("\n");
            body.push_str(
                "\n--- quick actions ---\n\
                 AddCol / DropCol / AddIdx / DropIdx / AddCon / DropCon\n\
                 (action bar → same review dialog as grid DDL)",
            );
            model.workbench_mut().inspector = crate::model::inspector::InspectorModel {
                open: true,
                title: format!("{schema}.{table} structure"),
                kind_label: "structure".into(),
                text: body,
                hex: String::new(),
                byte_len: columns.iter().map(|c| c.len() as u64).sum(),
                original_byte_len: None,
                stale: false,
                structure_schema: Some(schema),
                structure_table: Some(table),
            };
            Update::render()
        }
        Message::Engine(EngineMsg::RelationStructureFailed {
            context_revision,
            reason,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::TableOpDone {
            context_revision,
            op,
            schema,
            table,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.error_label = Some(format!("{op} {schema}.{table} done"));
            }
            if op == "drop" {
                // Table gone — clear base identity.
                if let Some(grid) = model.workbench_mut().active_grid_mut() {
                    grid.base_table = None;
                    grid.identity_columns.clear();
                    grid.cells.clear();
                    grid.row_count = 0;
                }
                return Update::render();
            }
            if op == "rename" {
                // Re-attach base table name from status if possible — operator
                // re-browses; clear identity until next browse proves PK again.
                if let Some(grid) = model.workbench_mut().active_grid_mut() {
                    grid.identity_columns.clear();
                    grid.base_table = None;
                }
                return Update::render();
            }
            rebrowse_active_table(model)
        }
        Message::Engine(EngineMsg::TableOpFailed {
            context_revision,
            reason,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::ActivitySnapshot {
            context_revision,
            lines,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            model.workbench_mut().inspector = crate::model::inspector::InspectorModel {
                open: true,
                title: "pg_stat_activity".into(),
                kind_label: "activity".into(),
                text: lines.join("\n"),
                hex: String::new(),
                byte_len: lines.iter().map(|l| l.len() as u64).sum(),
                original_byte_len: None,
                stale: false,
                structure_schema: None,
                structure_table: None,
            };
            Update::render()
        }
        Message::Engine(EngineMsg::ActivityFailed {
            context_revision,
            reason,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::RolesSnapshot {
            context_revision,
            lines,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            model.workbench_mut().inspector = crate::model::inspector::InspectorModel {
                open: true,
                title: "roles".into(),
                kind_label: "roles".into(),
                text: lines.join("\n"),
                hex: String::new(),
                byte_len: lines.iter().map(|l| l.len() as u64).sum(),
                original_byte_len: None,
                stale: false,
                structure_schema: None,
                structure_table: None,
            };
            Update::render()
        }
        Message::Engine(EngineMsg::RolesFailed {
            context_revision,
            reason,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::StartupReviewDone { summary, .. }) => {
            if let Some(mut session) = model.session().cloned() {
                session.status = Some(summary);
                model.set_session(Some(session));
            }
            Update::render()
        }
        Message::Engine(EngineMsg::ScriptSections {
            context_revision,
            lines,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            // Project summary into inspector; grid may also receive last page.
            model.workbench_mut().inspector = crate::model::inspector::InspectorModel {
                open: true,
                title: "script results".into(),
                kind_label: "script".into(),
                text: lines.join("\n"),
                hex: String::new(),
                byte_len: lines.iter().map(|l| l.len() as u64).sum(),
                original_byte_len: None,
                stale: false,
                structure_schema: None,
                structure_table: None,
            };
            let selected = model.workbench().selected_tab;
            if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
                tab.running = false;
            }
            Update::render()
        }
        Message::Engine(EngineMsg::RedisPipelineDone {
            context_revision,
            lines,
            ok_count,
            fail_count,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            // Build result sections from outcome lines (ordinal prefix "N. ").
            let mut sections = crate::model::result_sections::ResultSectionsModel::default();
            for (i, line) in lines.iter().enumerate() {
                let ordinal = (i + 1) as u32;
                let failed = line.contains(" ERR ");
                let tag = line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("CMD")
                    .trim_end_matches('(')
                    .to_owned();
                sections.push(crate::model::result_sections::StatementSection {
                    ordinal,
                    command_tag: tag,
                    kind: if failed {
                        crate::model::result_sections::StatementSectionKind::Failed
                    } else {
                        crate::model::result_sections::StatementSectionKind::Completed
                    },
                    rows: None,
                    elapsed_ms: None,
                    error: if failed {
                        Some(line.clone())
                    } else {
                        None
                    },
                    pinned: false,
                });
            }
            model.workbench_mut().result_sections = sections;
            model.workbench_mut().inspector = crate::model::inspector::InspectorModel {
                open: true,
                title: format!("redis pipeline {ok_count}ok/{fail_count}err"),
                kind_label: "redis-pipeline".into(),
                text: lines.join("\n"),
                hex: String::new(),
                byte_len: lines.iter().map(|l| l.len() as u64).sum(),
                original_byte_len: None,
                stale: false,
                structure_schema: None,
                structure_table: None,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if fail_count > 0 {
                    grid.mark_failed(format!("pipeline {ok_count}ok/{fail_count}err"));
                } else {
                    grid.mark_completed();
                    grid.error_label = Some(format!("pipeline {ok_count}ok"));
                }
            }
            let selected = model.workbench().selected_tab;
            if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
                tab.running = false;
            }
            Update::render()
        }
        Message::Engine(EngineMsg::RedisSubscribePage {
            context_revision,
            selector,
            pattern,
            lines,
            total_messages,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let mode = if pattern { "PSUBSCRIBE" } else { "SUBSCRIBE" };
            model.workbench_mut().inspector.open = true;
            model.workbench_mut().inspector.title =
                format!("{mode} {selector} · {total_messages} msg (live)");
            model.workbench_mut().inspector.kind_label = "pubsub".into();
            if !lines.is_empty() {
                let prev = model.workbench().inspector.text.clone();
                let chunk = lines.join("\n");
                model.workbench_mut().inspector.text = if prev.is_empty() {
                    chunk
                } else {
                    format!("{prev}\n{chunk}")
                };
            }
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.operation = GridOperationState::Streaming;
            }
            Update::render()
        }
        Message::Engine(EngineMsg::RedisSubscribeDone {
            context_revision,
            selector,
            pattern,
            lines,
            timed_out,
            idle_stop,
            cancelled,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let mode = if pattern { "PSUBSCRIBE" } else { "SUBSCRIBE" };
            let mut sections = crate::model::result_sections::ResultSectionsModel::default();
            for (i, _line) in lines.iter().enumerate() {
                sections.push(crate::model::result_sections::StatementSection {
                    ordinal: (i + 1) as u32,
                    command_tag: mode.into(),
                    kind: crate::model::result_sections::StatementSectionKind::Completed,
                    rows: Some(1),
                    elapsed_ms: None,
                    error: None,
                    pinned: false,
                });
            }
            if timed_out && lines.is_empty() {
                sections.push(crate::model::result_sections::StatementSection {
                    ordinal: 1,
                    command_tag: mode.into(),
                    kind: crate::model::result_sections::StatementSectionKind::Completed,
                    rows: Some(0),
                    elapsed_ms: None,
                    error: Some("wait timed out (no messages)".into()),
                    pinned: false,
                });
            }
            model.workbench_mut().result_sections = sections;
            model.workbench_mut().inspector.open = true;
            let suffix = if cancelled {
                " · cancelled"
            } else if idle_stop {
                " · idle stop"
            } else if timed_out {
                " · timeout"
            } else {
                ""
            };
            model.workbench_mut().inspector.title =
                format!("{mode} {selector} · {} msg{suffix}", lines.len());
            model.workbench_mut().inspector.kind_label = "pubsub".into();
            model.workbench_mut().inspector.text = lines.join("\n");
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if cancelled {
                    grid.mark_server_confirmed_cancelled();
                } else {
                    grid.operation = GridOperationState::Completed;
                }
            }
            let selected = model.workbench().selected_tab;
            if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
                tab.running = false;
            }
            Update::render()
        }
        Message::Engine(EngineMsg::RedisSubscribeFailed {
            context_revision,
            reason,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(l) => l,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            let selected = model.workbench().selected_tab;
            if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
                tab.running = false;
            }
            Update::render()
        }
        Message::Engine(EngineMsg::RedisPipelineFailed {
            context_revision,
            reason,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            let selected = model.workbench().selected_tab;
            if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
                tab.running = false;
            }
            Update::render()
        }
        Message::Engine(EngineMsg::PgToolDone {
            kind,
            summary,
            ok,
            ..
        }) => {
            if let Some(mut session) = model.session().cloned() {
                session.status = Some(format!(
                    "pg_{kind}: {} ({summary})",
                    if ok { "ok" } else { "failed" }
                ));
                model.set_session(Some(session));
            }
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if ok {
                    grid.error_label = Some(format!("pg_{kind}: {summary}"));
                } else {
                    grid.mark_failed(format!("pg_{kind}: {summary}"));
                }
            }
            Update::render()
        }
        Message::Engine(EngineMsg::BackendSignalDone {
            context_revision,
            kind,
            pid,
            acknowledged,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.error_label = Some(format!(
                    "{kind} backend {pid}: {}",
                    if acknowledged { "acked" } else { "not acked" }
                ));
            }
            Update::render()
        }
        Message::Engine(EngineMsg::BackendSignalFailed {
            context_revision,
            reason,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::MutationKillDone {
            context_revision,
            database,
            table,
            mutation_id,
            status_lines,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let mut lines = vec![format!(
                "KILL MUTATION {database}.{table} id={mutation_id}"
            )];
            lines.extend(status_lines);
            let text = lines.join("\n");
            model.workbench_mut().inspector = crate::model::inspector::InspectorModel {
                open: true,
                title: format!("mutation kill {database}.{table}"),
                kind_label: "mutation".into(),
                text: text.clone(),
                hex: String::new(),
                byte_len: text.len() as u64,
                original_byte_len: None,
                stale: false,
                structure_schema: Some(database.clone()),
                structure_table: Some(table.clone()),
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.error_label =
                    Some(format!("kill mutation {mutation_id} on {database}.{table}"));
            }
            Update::render()
        }
        Message::Engine(EngineMsg::MutationKillFailed {
            context_revision,
            reason,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::RedisKeysLoaded {
            context_revision,
            keys,
            has_more,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            use crate::model::redis_namespace::{group_by_namespace, project_key};
            let projected: Vec<_> = keys
                .iter()
                .map(|k| project_key(k.as_bytes()))
                .collect();
            let groups = group_by_namespace(&projected);
            let mut lines = vec![format!(
                "SCAN keys: {} (more={has_more})",
                keys.len()
            )];
            for (ns, idxs) in groups {
                lines.push(format!("namespace {ns}: {} keys", idxs.len()));
                for i in idxs.into_iter().take(32) {
                    lines.push(format!("  {}", projected[i].full));
                }
            }
            model.workbench_mut().inspector = crate::model::inspector::InspectorModel {
                open: true,
                title: "redis SCAN".into(),
                kind_label: "keys".into(),
                text: lines.join("\n"),
                hex: String::new(),
                byte_len: keys.iter().map(|k| k.len() as u64).sum(),
                original_byte_len: None,
                stale: false,
                structure_schema: None,
                structure_table: None,
            };
            Update::render()
        }
        Message::Engine(EngineMsg::RedisKeysFailed {
            context_revision,
            reason,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::RedisKeyViewLoaded {
            context_revision,
            key,
            kind_label,
            lines,
            next_collection_skip,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let logical_db = model.workbench().context.database.clone();
            // Clear staged collection edits when the open key changes.
            let key_changed = model
                .workbench()
                .redis_stage_target
                .as_ref()
                .is_none_or(|(_, k, _)| k != &key);
            if key_changed {
                model.workbench_mut().redis_staged.clear();
            }
            model.workbench_mut().redis_stage_target =
                Some((logical_db.clone(), key.clone(), kind_label.clone()));
            model.workbench_mut().redis_collection_skip = next_collection_skip;
            model.workbench_mut().inspector = crate::model::inspector::InspectorModel {
                open: true,
                title: format!("redis:{key}"),
                kind_label: kind_label.clone(),
                text: lines.join("\n"),
                hex: String::new(),
                byte_len: lines.iter().map(|l| l.len() as u64).sum(),
                original_byte_len: None,
                stale: false,
                structure_schema: Some(logical_db),
                structure_table: Some(key),
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_completed();
            }
            Update::render()
        }
        Message::Engine(EngineMsg::RedisKeyViewFailed {
            context_revision,
            reason,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::RedisInfoLoaded {
            context_revision,
            sampled_at_ms,
            lines,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            model.workbench_mut().inspector = crate::model::inspector::InspectorModel {
                open: true,
                title: format!("redis INFO @ {sampled_at_ms}"),
                kind_label: "info".into(),
                text: lines.join("\n"),
                hex: String::new(),
                byte_len: lines.iter().map(|l| l.len() as u64).sum(),
                original_byte_len: None,
                stale: false,
                structure_schema: None,
                structure_table: None,
            };
            Update::render()
        }
        Message::Engine(EngineMsg::RedisInfoFailed {
            context_revision,
            reason,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::ExportDone { path, bytes, .. }) => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.error_label = Some(format!("exported {bytes} B → {path}"));
            }
            Update::render()
        }
        Message::Engine(EngineMsg::ExportFailed {
            reason,
            partial_removed,
            ..
        }) => {
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(format!(
                    "export failed: {label} (partial_removed={partial_removed})"
                ));
            }
            Update::render()
        }
        Message::Engine(EngineMsg::GridFailed {
            context_revision,
            reason,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            if model
                .workbench()
                .active_grid()
                .is_some_and(|g| g.operation == GridOperationState::Disconnected)
            {
                return Update::unchanged();
            }
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(label);
            }
            let selected = model.workbench().selected_tab;
            if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
                tab.running = false;
            }
            Update::render()
        }
        Message::Engine(EngineMsg::GridCancelDispatched { dispatch, .. }) => {
            if model
                .workbench()
                .active_grid()
                .is_some_and(|g| g.operation == GridOperationState::Disconnected)
            {
                return Update::unchanged();
            }
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_cancel_dispatch(&dispatch);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::GridCancelled { label, .. }) => {
            if model
                .workbench()
                .active_grid()
                .is_some_and(|g| g.operation == GridOperationState::Disconnected)
            {
                return Update::unchanged();
            }
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if label.contains("server confirmed") {
                    grid.mark_server_confirmed_cancelled();
                } else if label.contains("unknown") {
                    grid.operation = GridOperationState::CancelUnknown;
                } else {
                    grid.mark_cancelled();
                }
                grid.error_label = Some(label);
            }
            let selected = model.workbench().selected_tab;
            if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
                tab.running = false;
            }
            Update::render()
        }
        Message::Engine(EngineMsg::ConnectFailed { reason, .. }) => {
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            model.editor_mut().test_status = Some(format!("connect failed: {label}"));
            if let Some(session) = model.session().cloned() {
                let mut session = session;
                session.status = Some(format!("failed: {label}"));
                model.set_session(Some(session));
            }
            Update::render()
        }
        Message::Engine(EngineMsg::DisconnectOk { .. }) => {
            model.set_session(None);
            model.set_screen(Screen::Connections);
            model.set_action(ActionId::Open);
            Update::render()
        }
        Message::Engine(EngineMsg::DisconnectFailed { reason, .. }) => {
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(mut session) = model.session().cloned() {
                session.status = Some(format!("disconnect failed: {label}"));
                model.set_session(Some(session));
            }
            Update::render()
        }
        Message::Engine(EngineMsg::PasswordPromptRequired {
            request_token,
            profile_id_hex,
        }) => {
            model.set_password_prompt(Some(PasswordPrompt {
                request_token,
                profile_id_hex,
                buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        Message::Engine(EngineMsg::Reconnecting {
            request_token,
            attempt,
            next_delay_ms,
            draft,
            ..
        }) => {
            if let Some(mut session) = model.session().cloned() {
                session.status = Some(format!(
                    "reconnecting attempt {attempt} (next {next_delay_ms} ms)"
                ));
                model.set_session(Some(session));
            }
            // Auto re-dispatch: executor sleeps next_delay_ms when attempt > 0.
            Update {
                render: true,
                effect: Some(Effect::ReconnectSession {
                    request_token,
                    draft,
                    attempt,
                }),
            }
        }
        Message::Engine(EngineMsg::ReconnectStopped { reason, .. }) => {
            let label = match reason {
                FailureProjection::Label(label) => label,
            };
            if let Some(mut session) = model.session().cloned() {
                session.status = Some(format!("reconnect stopped: {label}"));
                model.set_session(Some(session));
            }
            Update::render()
        }
        Message::FocusNext => {
            if model.move_focus(false) {
                Update::render()
            } else {
                Update::unchanged()
            }
        }
        Message::FocusPrevious => {
            if model.move_focus(true) {
                Update::render()
            } else {
                Update::unchanged()
            }
        }
        Message::ActionNext | Message::ActionPrevious
            if model.focus() == Some(FocusRegion::Actions) =>
        {
            let reverse = matches!(message, Message::ActionPrevious);
            model.set_action(cycle_action(
                model.screen(),
                model.selected_action(),
                reverse,
                model.password_prompt().is_some() || model.confirm().is_some(),
            ));
            Update::render()
        }
        Message::ActionNext | Message::ActionPrevious => Update::unchanged(),
        Message::Activate if model.focus() == Some(FocusRegion::Actions) => {
            activate_selected_action(model)
        }
        Message::Activate
            if model.screen() == Screen::Connections
                && model.focus() == Some(FocusRegion::Content) =>
        {
            model.profiles_mut().select_next();
            Update::render()
        }
        Message::Activate
            if model.screen() == Screen::Workbench
                && model.focus() == Some(FocusRegion::Catalog) =>
        {
            activate_catalog_node(model)
        }
        Message::Activate if model.screen() == Screen::Editor => {
            model.editor_mut().focus_next();
            Update::render()
        }
        Message::Activate
            if model.screen() == Screen::Workbench
                && model.focus() == Some(FocusRegion::Content)
                && model
                    .workbench()
                    .active_grid()
                    .is_some_and(|g| g.cell_edit.is_some()) =>
        {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if grid.commit_cell_edit() {
                    model.workbench_mut().mark_active_dirty(true);
                }
            }
            Update::render()
        }
        Message::Activate => Update::unchanged(),
        Message::RequestRedraw => Update::render(),
        Message::Quit => Update::with_effect(Effect::Exit),
    }
}

fn safety_mode_from_label(label: &str) -> tablerock_core::ProfileSafetyMode {
    let lower = label.to_ascii_lowercase();
    if lower.contains("read only") || lower.contains("readonly") {
        tablerock_core::ProfileSafetyMode::ReadOnly
    } else {
        tablerock_core::ProfileSafetyMode::ConfirmWrites
    }
}

fn focus_target(model: &mut Model, target: ShellTarget) {
    match target {
        ShellTarget::Focus(focus) => {
            let _ = model.request_focus(focus);
        }
        ShellTarget::Action(action) => {
            let _ = model.request_focus(FocusRegion::Actions);
            model.set_action(action);
        }
    }
}

fn activate_selected_action(model: &mut Model) -> Update {
    match model.selected_action() {
        ActionId::Submit if model.password_prompt().is_some() => {
            let Some(prompt) = model.password_prompt().cloned() else {
                return Update::unchanged();
            };
            model.set_password_prompt(None);
            Update {
                render: true,
                effect: Some(Effect::ResumeConnectProfile {
                    request_token: prompt.request_token,
                    profile_id_hex: prompt.profile_id_hex,
                    password: prompt.buffer,
                }),
            }
        }
        ActionId::Submit if model.confirm().is_some() => {
            let Some(confirm) = model.confirm().cloned() else {
                return Update::unchanged();
            };
            match confirm {
                ConfirmDialog::RemoveProfile { id_hex, .. } => {
                    let token = model.mint_request_token();
                    model.set_profiles(ProfileListState::Loading {
                        request_token: token,
                    });
                    model.set_confirm(None);
                    Update {
                        render: true,
                        effect: Some(Effect::DeleteProfile {
                            request_token: token,
                            profile_id_hex: id_hex,
                        }),
                    }
                }
                ConfirmDialog::RemoveGroup { name } => {
                    let token = model.mint_request_token();
                    model.set_profiles(ProfileListState::Loading {
                        request_token: token,
                    });
                    model.set_confirm(None);
                    Update {
                        render: true,
                        effect: Some(Effect::DeleteGroup {
                            request_token: token,
                            group_name: name,
                        }),
                    }
                }
                ConfirmDialog::RenameGroup {
                    old_name,
                    confirm_buffer,
                } => {
                    let new_name = confirm_buffer.trim().to_owned();
                    if !is_safe_group_name(&new_name) || new_name == old_name {
                        return Update::render();
                    }
                    let token = model.mint_request_token();
                    model.set_profiles(ProfileListState::Loading {
                        request_token: token,
                    });
                    model.set_confirm(None);
                    Update {
                        render: true,
                        effect: Some(Effect::RenameGroup {
                            request_token: token,
                            old_name,
                            new_name,
                        }),
                    }
                }
                ConfirmDialog::CloseDirtyTab { index, .. } => {
                    model.set_confirm(None);
                    model.workbench_mut().force_close_tab(index);
                    model.set_action(ActionId::Disconnect);
                    Update::render()
                }
                ConfirmDialog::TruncateTable {
                    schema,
                    table,
                    confirm_buffer,
                } => {
                    // Fail closed: buffer must equal the table name exactly.
                    if confirm_buffer != table {
                        return Update::render();
                    }
                    let Some(session_id_hex) =
                        model.session().map(|s| s.session_id_hex.clone())
                    else {
                        model.set_confirm(None);
                        return Update::unchanged();
                    };
                    let token = model.mint_request_token();
                    let context_revision = model.workbench().context_revision;
                    model.set_confirm(None);
                    Update {
                        render: true,
                        effect: Some(Effect::ExecuteTableOp {
                            request_token: token,
                            session_id_hex,
                            context_revision,
                            op: "truncate".into(),
                            schema,
                            table,
                            new_table: String::new(),
                        }),
                    }
                }
                ConfirmDialog::DropTable {
                    schema,
                    table,
                    confirm_buffer,
                } => {
                    if confirm_buffer != table {
                        return Update::render();
                    }
                    let Some(session_id_hex) =
                        model.session().map(|s| s.session_id_hex.clone())
                    else {
                        model.set_confirm(None);
                        return Update::unchanged();
                    };
                    let token = model.mint_request_token();
                    let context_revision = model.workbench().context_revision;
                    model.set_confirm(None);
                    Update {
                        render: true,
                        effect: Some(Effect::ExecuteTableOp {
                            request_token: token,
                            session_id_hex,
                            context_revision,
                            op: "drop".into(),
                            schema,
                            table,
                            new_table: String::new(),
                        }),
                    }
                }
                ConfirmDialog::VacuumTable {
                    schema,
                    table,
                    confirm_buffer,
                } => {
                    if confirm_buffer != table {
                        return Update::render();
                    }
                    let Some(session_id_hex) =
                        model.session().map(|s| s.session_id_hex.clone())
                    else {
                        model.set_confirm(None);
                        return Update::unchanged();
                    };
                    let token = model.mint_request_token();
                    let context_revision = model.workbench().context_revision;
                    model.set_confirm(None);
                    Update {
                        render: true,
                        effect: Some(Effect::ExecuteTableOp {
                            request_token: token,
                            session_id_hex,
                            context_revision,
                            op: "vacuum".into(),
                            schema,
                            table,
                            new_table: String::new(),
                        }),
                    }
                }
                ConfirmDialog::AnalyzeTable {
                    schema,
                    table,
                    confirm_buffer,
                } => {
                    if confirm_buffer != table {
                        return Update::render();
                    }
                    let Some(session_id_hex) =
                        model.session().map(|s| s.session_id_hex.clone())
                    else {
                        model.set_confirm(None);
                        return Update::unchanged();
                    };
                    let token = model.mint_request_token();
                    let context_revision = model.workbench().context_revision;
                    model.set_confirm(None);
                    Update {
                        render: true,
                        effect: Some(Effect::ExecuteTableOp {
                            request_token: token,
                            session_id_hex,
                            context_revision,
                            op: "analyze".into(),
                            schema,
                            table,
                            new_table: String::new(),
                        }),
                    }
                }
                ConfirmDialog::OptimizeTable {
                    schema,
                    table,
                    confirm_buffer,
                } => {
                    if confirm_buffer != table {
                        return Update::render();
                    }
                    let Some(session_id_hex) =
                        model.session().map(|s| s.session_id_hex.clone())
                    else {
                        model.set_confirm(None);
                        return Update::unchanged();
                    };
                    let token = model.mint_request_token();
                    let context_revision = model.workbench().context_revision;
                    model.set_confirm(None);
                    Update {
                        render: true,
                        effect: Some(Effect::ExecuteTableOp {
                            request_token: token,
                            session_id_hex,
                            context_revision,
                            op: "optimize".into(),
                            schema,
                            table,
                            new_table: String::new(),
                        }),
                    }
                }
                ConfirmDialog::DdlReview {
                    kind,
                    schema,
                    table,
                    confirm_buffer,
                    ..
                } => {
                    let parts: Vec<&str> = confirm_buffer.split_whitespace().collect();
                    let (object_name, type_text) = match kind.as_str() {
                        "add_column" | "create_index" | "add_constraint"
                            if parts.len() >= 2 =>
                        {
                            (parts[0].to_owned(), parts[1..].join(" "))
                        }
                        "drop_column" | "drop_index" | "drop_constraint"
                            if parts.len() == 1 =>
                        {
                            (parts[0].to_owned(), String::new())
                        }
                        _ => return Update::render(),
                    };
                    if object_name.is_empty() {
                        return Update::render();
                    }
                    let Some(session_id_hex) =
                        model.session().map(|s| s.session_id_hex.clone())
                    else {
                        model.set_confirm(None);
                        return Update::unchanged();
                    };
                    let token = model.mint_request_token();
                    let context_revision = model.workbench().context_revision;
                    model.set_confirm(None);
                    Update {
                        render: true,
                        effect: Some(Effect::ExecuteDdlPlan {
                            request_token: token,
                            session_id_hex,
                            context_revision,
                            kind,
                            schema,
                            table,
                            object_name,
                            type_text,
                        }),
                    }
                }
                ConfirmDialog::StartupReview {
                    items,
                    confirm_buffer,
                } => {
                    if confirm_buffer.trim() != "RUN" {
                        return Update::render();
                    }
                    if items.is_empty() {
                        model.set_confirm(None);
                        return Update::render();
                    }
                    let Some(session_id_hex) =
                        model.session().map(|s| s.session_id_hex.clone())
                    else {
                        model.set_confirm(None);
                        return Update::unchanged();
                    };
                    let token = model.mint_request_token();
                    let context_revision = model.workbench().context_revision;
                    model.set_confirm(None);
                    Update {
                        render: true,
                        effect: Some(Effect::ExecuteStartupReviewed {
                            request_token: token,
                            session_id_hex,
                            context_revision,
                            items,
                        }),
                    }
                }
                ConfirmDialog::PgTool {
                    kind,
                    confirm_buffer,
                } => {
                    let path = if confirm_buffer.trim().is_empty() {
                        "tablerock.dump".to_owned()
                    } else {
                        confirm_buffer.trim().to_owned()
                    };
                    let editor = model.editor();
                    let host = editor.host.clone();
                    let port: u16 = editor.port.parse().unwrap_or(5432);
                    let database = if editor.database.is_empty() {
                        "postgres".into()
                    } else {
                        editor.database.clone()
                    };
                    let username = if editor.username.is_empty() {
                        "postgres".into()
                    } else {
                        editor.username.clone()
                    };
                    let password = editor.password.clone();
                    let token = model.mint_request_token();
                    model.set_confirm(None);
                    match kind.as_str() {
                        "dump" => Update {
                            render: true,
                            effect: Some(Effect::RunPgDump {
                                request_token: token,
                                host,
                                port,
                                database,
                                username,
                                password,
                                path,
                                tool_path: String::new(),
                            }),
                        },
                        "restore" => Update {
                            render: true,
                            effect: Some(Effect::RunPgRestore {
                                request_token: token,
                                host,
                                port,
                                database,
                                username,
                                password,
                                path,
                                tool_path: String::new(),
                            }),
                        },
                        _ => Update::render(),
                    }
                }
                ConfirmDialog::ImportUrl { confirm_buffer } => {
                    let url = confirm_buffer.trim();
                    if url.is_empty() {
                        return Update::render();
                    }
                    match tablerock_core::parse_connection_url(url) {
                        Ok(draft) => {
                            model.editor_mut().apply_connection_url(&draft);
                            model.set_confirm(None);
                            model.set_screen(Screen::Editor);
                            model.set_action(ActionId::Test);
                            Update::render()
                        }
                        Err(error) => {
                            model.editor_mut().validation_error = Some(error.to_string());
                            model.set_confirm(None);
                            model.set_screen(Screen::Editor);
                            Update::render()
                        }
                    }
                }
                ConfirmDialog::OpenExternalUrl {
                    url,
                    matched_profile_id_hex,
                    confirm_buffer,
                    ..
                } => {
                    // Two-phase: empty buffer receives URL paste; OPEN confirms connect.
                    let trimmed = confirm_buffer.trim();
                    if trimmed.eq_ignore_ascii_case("OPEN") || trimmed.eq_ignore_ascii_case("YES") {
                        if url.trim().is_empty() {
                            return Update::render();
                        }
                        // Prefer matched saved profile when present.
                        if let Some(profile_id_hex) = matched_profile_id_hex {
                            model.set_confirm(None);
                            let token = model.mint_request_token();
                            return Update {
                                render: true,
                                effect: Some(Effect::ConnectProfile {
                                    request_token: token,
                                    profile_id_hex,
                                }),
                            };
                        }
                        match tablerock_core::parse_connection_url(&url) {
                            Ok(draft) => {
                                model.reset_editor();
                                model.editor_mut().apply_connection_url(&draft);
                                model.set_confirm(None);
                                model.set_screen(Screen::Editor);
                                if !model.editor_mut().validate() {
                                    return Update::render();
                                }
                                let token = model.mint_request_token();
                                Update {
                                    render: true,
                                    effect: Some(Effect::ConnectSession {
                                        request_token: token,
                                        draft: connection_draft_from_editor(model.editor()),
                                        temporary: true,
                                    }),
                                }
                            }
                            Err(error) => {
                                model.set_confirm(None);
                                model.editor_mut().validation_error = Some(error.to_string());
                                model.set_screen(Screen::Editor);
                                Update::render()
                            }
                        }
                    } else if trimmed.contains("://") {
                        // First paste: parse URL into summary, keep for OPEN confirm.
                        match tablerock_core::parse_connection_url(trimmed) {
                            Ok(draft) => {
                                let mut summary = draft.safety_summary();
                                if let Some(secret) = draft.password.as_deref() {
                                    if !secret.is_empty() && summary.contains(secret) {
                                        model.set_confirm(None);
                                        model.editor_mut().validation_error =
                                            Some("refusing to show password in summary".into());
                                        return Update::render();
                                    }
                                }
                                let engine_label = match draft.engine {
                                    tablerock_core::Engine::PostgreSql => "PostgreSQL",
                                    tablerock_core::Engine::ClickHouse => "ClickHouse",
                                    tablerock_core::Engine::Redis => "Redis",
                                };
                                let matched = match model.profiles() {
                                    crate::model::profiles::ProfileListState::Loaded {
                                        rows,
                                        ..
                                    } => rows.iter().find(|r| {
                                        r.matches_url_target(
                                            engine_label,
                                            &draft.host,
                                            draft.port,
                                            &draft.database,
                                        )
                                    }),
                                    _ => None,
                                };
                                let matched_profile_id_hex =
                                    matched.map(|r| r.id_hex.clone());
                                if let Some(row) = matched {
                                    summary.push_str(&format!(
                                        " · matched saved profile '{}'",
                                        row.name
                                    ));
                                } else {
                                    summary.push_str(" · temporary session (no saved match)");
                                }
                                model.set_confirm(Some(ConfirmDialog::OpenExternalUrl {
                                    url: trimmed.to_owned(),
                                    summary,
                                    matched_profile_id_hex,
                                    confirm_buffer: String::new(),
                                }));
                                model.set_action(ActionId::Submit);
                                Update::render()
                            }
                            Err(error) => {
                                model.set_confirm(None);
                                model.editor_mut().validation_error = Some(error.to_string());
                                Update::render()
                            }
                        }
                    } else {
                        // Stay open for OPEN token.
                        Update::render()
                    }
                }
                ConfirmDialog::QuickSwitch { confirm_buffer } => {
                    apply_quick_switch(model, confirm_buffer.trim())
                }
                ConfirmDialog::FindReplace { confirm_buffer } => {
                    let raw = confirm_buffer.trim();
                    if raw.is_empty() {
                        return Update::render();
                    }
                    // Formats: find=>replace | find=>replace=>all | find=>replace=>i | find=>replace=>all=>i
                    let parts: Vec<&str> = raw.split("=>").map(str::trim).collect();
                    if parts.len() < 2 || parts[0].is_empty() {
                        return Update::render();
                    }
                    let needle = parts[0];
                    let replacement = parts[1];
                    let flags = parts.get(2..).unwrap_or(&[]);
                    let all = flags.iter().any(|f| f.eq_ignore_ascii_case("all"));
                    let case_i = flags.iter().any(|f| {
                        f.eq_ignore_ascii_case("i") || f.eq_ignore_ascii_case("ci")
                    });
                    let Some(ed) = model.workbench_mut().active_editor_mut() else {
                        model.set_confirm(None);
                        return Update::unchanged();
                    };
                    let n = if all {
                        ed.replace_all(needle, replacement, case_i)
                    } else if ed.replace_next(needle, replacement, case_i) {
                        1
                    } else {
                        0
                    };
                    model.set_confirm(None);
                    if let Some(g) = model.workbench_mut().active_grid_mut() {
                        g.error_label = Some(format!("replace: {n} occurrence(s)"));
                    }
                    Update::render()
                }
                ConfirmDialog::BindParams {
                    names,
                    statement,
                    confirm_buffer,
                } => {
                    use tablerock_core::{
                        bind_named_values, parse_param_bindings, rewrite_named_params,
                    };
                    let Some(session_id_hex) =
                        model.session().map(|s| s.session_id_hex.clone())
                    else {
                        model.set_confirm(None);
                        return Update::unchanged();
                    };
                    let plan = match rewrite_named_params(&statement) {
                        Ok(p) => p,
                        Err(e) => {
                            model.set_confirm(None);
                            if let Some(g) = model.workbench_mut().active_grid_mut() {
                                g.mark_failed(e.to_string());
                            }
                            return Update::render();
                        }
                    };
                    let bindings = parse_param_bindings(&confirm_buffer);
                    match bind_named_values(&plan, &bindings) {
                        Ok(values) => {
                            model.set_confirm(None);
                            emit_execute_sql(model, session_id_hex, plan.sql, values)
                        }
                        Err(missing) => {
                            // Stay open; show which names remain.
                            let _ = names;
                            if let Some(ConfirmDialog::BindParams {
                                confirm_buffer: buf,
                                ..
                            }) = model.confirm_mut()
                            {
                                *buf = format!(
                                    "# missing: {}\n{}",
                                    missing.join(", "),
                                    confirm_buffer
                                );
                            }
                            Update::render()
                        }
                    }
                }
                ConfirmDialog::RenameTable {
                    schema,
                    table,
                    confirm_buffer,
                } => {
                    let new_name = confirm_buffer.trim();
                    if new_name.is_empty() || new_name == table {
                        return Update::render();
                    }
                    let Some(session_id_hex) =
                        model.session().map(|s| s.session_id_hex.clone())
                    else {
                        model.set_confirm(None);
                        return Update::unchanged();
                    };
                    let token = model.mint_request_token();
                    let context_revision = model.workbench().context_revision;
                    model.set_confirm(None);
                    Update {
                        render: true,
                        effect: Some(Effect::ExecuteTableOp {
                            request_token: token,
                            session_id_hex,
                            context_revision,
                            op: "rename".into(),
                            schema,
                            table,
                            new_table: new_name.to_owned(),
                        }),
                    }
                }
                ConfirmDialog::CancelBackend {
                    ref confirm_buffer,
                    ..
                }
                | ConfirmDialog::TerminateBackend {
                    ref confirm_buffer,
                    ..
                } => {
                    let trimmed = confirm_buffer.trim();
                    let Ok(pid) = trimmed.parse::<i32>() else {
                        return Update::render();
                    };
                    if pid <= 0 {
                        return Update::render();
                    }
                    let kind = match &confirm {
                        ConfirmDialog::CancelBackend { .. } => "cancel",
                        ConfirmDialog::TerminateBackend { .. } => "terminate",
                        _ => unreachable!(),
                    };
                    let Some(session_id_hex) =
                        model.session().map(|s| s.session_id_hex.clone())
                    else {
                        model.set_confirm(None);
                        return Update::unchanged();
                    };
                    let token = model.mint_request_token();
                    let context_revision = model.workbench().context_revision;
                    model.set_confirm(None);
                    Update {
                        render: true,
                        effect: Some(Effect::SignalBackend {
                            request_token: token,
                            session_id_hex,
                            context_revision,
                            kind: kind.into(),
                            pid,
                        }),
                    }
                }
                ConfirmDialog::KillMutation {
                    database,
                    table,
                    confirm_buffer,
                } => {
                    let mutation_id = confirm_buffer.trim().to_owned();
                    // Mirror engine gate: `mutation_2.txt` style ids only.
                    if mutation_id.is_empty()
                        || mutation_id.len() > 128
                        || !mutation_id.bytes().all(|b| {
                            b.is_ascii_alphanumeric()
                                || b == b'_'
                                || b == b'-'
                                || b == b'.'
                        })
                    {
                        return Update::render();
                    }
                    let Some(session_id_hex) =
                        model.session().map(|s| s.session_id_hex.clone())
                    else {
                        model.set_confirm(None);
                        return Update::unchanged();
                    };
                    let token = model.mint_request_token();
                    let context_revision = model.workbench().context_revision;
                    model.set_confirm(None);
                    Update {
                        render: true,
                        effect: Some(Effect::KillClickHouseMutation {
                            request_token: token,
                            session_id_hex,
                            context_revision,
                            database,
                            table,
                            mutation_id,
                        }),
                    }
                }
                ConfirmDialog::SaveFilter {
                    schema,
                    table,
                    confirm_buffer,
                } => {
                    let name = confirm_buffer.trim().to_owned();
                    if !crate::model::saved_filter::is_safe_preset_name(&name) {
                        return Update::render();
                    }
                    let Some(profile_id_hex) = model.workbench().profile_id_hex.clone() else {
                        model.set_confirm(None);
                        return Update::unchanged();
                    };
                    let (filters, raw_where) = {
                        let Some(grid) = model.workbench().active_grid() else {
                            model.set_confirm(None);
                            return Update::unchanged();
                        };
                        (grid.filters.clone(), grid.raw_where.clone())
                    };
                    model.workbench_mut().filter_library.upsert(
                        crate::model::saved_filter::SavedFilterPreset {
                            name,
                            schema,
                            table,
                            filters,
                            raw_where,
                        },
                    );
                    let library_json = model.workbench().filter_library.to_json();
                    let token = model.mint_request_token();
                    model.set_confirm(None);
                    Update {
                        render: true,
                        effect: Some(Effect::SaveSavedFilterLibrary {
                            request_token: token,
                            profile_id_hex,
                            library_json,
                        }),
                    }
                }
                ConfirmDialog::ApplyFilter {
                    schema,
                    table,
                    known_names,
                    confirm_buffer,
                } => {
                    let Some(name) = crate::model::saved_filter::resolve_preset_name(
                        &known_names,
                        &confirm_buffer,
                    ) else {
                        // Ambiguous / empty / unsafe: keep dialog open for refine.
                        return Update::render();
                    };
                    let preset = model
                        .workbench()
                        .filter_library
                        .get(&name, &schema, &table)
                        .cloned();
                    model.set_confirm(None);
                    let Some(preset) = preset else {
                        if let Some(grid) = model.workbench_mut().active_grid_mut() {
                            grid.error_label = Some(format!("no filter preset '{name}'"));
                        }
                        return Update::render();
                    };
                    if let Some(grid) = model.workbench_mut().active_grid_mut() {
                        grid.filters = preset.filters;
                        grid.raw_where = preset.raw_where;
                    }
                    rebrowse_active_table(model)
                }
                ConfirmDialog::StageRedis {
                    op,
                    logical_db,
                    key,
                    confirm_buffer,
                } => {
                    let Some(spec) =
                        crate::model::redis_stage::parse_stage_buffer(&op, &confirm_buffer)
                    else {
                        return Update::render();
                    };
                    // Ensure target matches the dialog (fail closed if key switched).
                    let target_ok = model.workbench().redis_stage_target.as_ref().is_some_and(
                        |(db, k, _)| db == &logical_db && k == &key,
                    );
                    if !target_ok {
                        model.set_confirm(None);
                        return Update::unchanged();
                    }
                    model.workbench_mut().redis_staged.push(spec);
                    let n = model.workbench().redis_staged.len();
                    model.workbench_mut().mark_active_dirty(true);
                    if let Some(grid) = model.workbench_mut().active_grid_mut() {
                        grid.error_label = Some(format!("staged redis {op} ({n} pending)"));
                    }
                    model.set_confirm(None);
                    Update::render()
                }
                ConfirmDialog::RedisSubscribe {
                    pattern,
                    confirm_buffer,
                } => {
                    let selector = confirm_buffer.trim().to_owned();
                    if selector.is_empty() || selector.len() > 256 {
                        return Update::render();
                    }
                    // Fail closed: printable channel/pattern only (no controls).
                    if selector.chars().any(|c| c.is_control()) {
                        return Update::render();
                    }
                    let Some(session_id_hex) =
                        model.session().map(|s| s.session_id_hex.clone())
                    else {
                        model.set_confirm(None);
                        return Update::unchanged();
                    };
                    let token = model.mint_request_token();
                    let context_revision = model.workbench().context_revision;
                    if let Some(grid) = model.workbench_mut().active_grid_mut() {
                        grid.operation = GridOperationState::Running;
                        grid.error_label = None;
                    }
                    model.set_confirm(None);
                    Update {
                        render: true,
                        effect: Some(Effect::RedisSubscribe {
                            request_token: token,
                            session_id_hex,
                            context_revision,
                            selector,
                            pattern,
                        }),
                    }
                }
            }
        }
        ActionId::Cancel if model.password_prompt().is_some() => {
            model.set_password_prompt(None);
            Update::render()
        }
        ActionId::Cancel if model.confirm().is_some() => {
            model.set_confirm(None);
            model.set_action(ActionId::Open);
            Update::render()
        }
        ActionId::Remove if model.screen() == Screen::Connections => {
            // Group branch selection (g:name) → remove group.
            if let Some(group) = selected_connection_group(model) {
                model.set_confirm(Some(ConfirmDialog::RemoveGroup { name: group }));
                model.set_action(ActionId::Submit);
                return Update::render();
            }
            if let Some(row) = model.profiles().selected_row() {
                if model.session().is_some() {
                    // Active session present: still ask (spec: ask when active sessions).
                }
                model.set_confirm(Some(ConfirmDialog::RemoveProfile {
                    id_hex: row.id_hex.clone(),
                    name: row.name.clone(),
                }));
                model.set_action(ActionId::Submit);
                return Update::render();
            }
            Update::unchanged()
        }
        ActionId::RenameGroup if model.screen() == Screen::Connections => {
            let Some(old_name) = selected_connection_group(model) else {
                return Update::unchanged();
            };
            model.set_confirm(Some(ConfirmDialog::RenameGroup {
                old_name,
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::Reconnect
            if matches!(
                model.screen(),
                Screen::Workbench | Screen::Editor | Screen::Connections
            ) =>
        {
            // Bounded reconnect from last connect draft or current editor.
            let draft = model
                .last_connect_draft
                .clone()
                .unwrap_or_else(|| connection_draft_from_editor(model.editor()));
            model.last_connect_draft = Some(draft.clone());
            let token = model.mint_request_token();
            if let Some(mut session) = model.session().cloned() {
                session.status = Some("reconnecting attempt 0".into());
                model.set_session(Some(session));
            }
            Update {
                render: true,
                effect: Some(Effect::ReconnectSession {
                    request_token: token,
                    draft,
                    attempt: 0,
                }),
            }
        }
        ActionId::SessionHealth if model.screen() == Screen::Workbench => {
            let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
                return Update::unchanged();
            };
            let token = model.mint_request_token();
            Update {
                render: true,
                effect: Some(Effect::CheckSessionHealth {
                    request_token: token,
                    session_id_hex,
                }),
            }
        }
        ActionId::Open if model.screen() == Screen::Connections => {
            let Some(profile_id_hex) = model
                .profiles()
                .selected_row()
                .map(|row| row.id_hex.clone())
            else {
                return Update::unchanged();
            };
            let token = model.mint_request_token();
            Update {
                render: true,
                effect: Some(Effect::ConnectProfile {
                    request_token: token,
                    profile_id_hex,
                }),
            }
        }
        ActionId::Open => {
            model.set_screen(Screen::ConnectionPicker);
            Update::render()
        }
        ActionId::New => {
            model.reset_editor();
            model.set_screen(Screen::Editor);
            model.set_action(ActionId::Save);
            Update::render()
        }
        ActionId::ImportUrl
            if matches!(
                model.screen(),
                Screen::Connections | Screen::ConnectionPicker | Screen::Editor
            ) =>
        {
            model.set_confirm(Some(ConfirmDialog::ImportUrl {
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::OpenExternalUrl
            if matches!(
                model.screen(),
                Screen::Connections | Screen::ConnectionPicker | Screen::Editor
            ) =>
        {
            model.set_confirm(Some(ConfirmDialog::OpenExternalUrl {
                url: String::new(),
                summary: String::new(),
                matched_profile_id_hex: None,
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::Save if model.screen() == Screen::Editor => {
            if !model.editor_mut().validate() {
                return Update::render();
            }
            let token = model.mint_request_token();
            model.set_profiles(ProfileListState::Loading {
                request_token: token,
            });
            // Persist via CLI; reload list after save completes (reuse Load token).
            Update {
                render: true,
                effect: Some(Effect::SaveConnection {
                    request_token: token,
                    draft: connection_draft_from_editor(model.editor()),
                }),
            }
        }
        ActionId::Test if model.screen() == Screen::Editor => {
            if !model.editor_mut().validate() {
                return Update::render();
            }
            let token = model.mint_request_token();
            model.editor_mut().test_status = Some("testing…".into());
            Update {
                render: true,
                effect: Some(Effect::TestConnection {
                    request_token: token,
                    draft: connection_draft_from_editor(model.editor()),
                }),
            }
        }
        ActionId::Connect if model.screen() == Screen::Editor => {
            if !model.editor_mut().validate() {
                return Update::render();
            }
            let token = model.mint_request_token();
            model.editor_mut().test_status = Some("connecting…".into());
            let draft = connection_draft_from_editor(model.editor());
            model.last_connect_draft = Some(draft.clone());
            Update {
                render: true,
                effect: Some(Effect::ConnectSession {
                    request_token: token,
                    draft,
                    // Editor Connect is temporary until list-row Open saves first.
                    temporary: true,
                }),
            }
        }
        ActionId::Disconnect if model.screen() == Screen::Workbench => {
            let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
                return Update::unchanged();
            };
            let token = model.mint_request_token();
            if let Some(mut facts) = model.session().cloned() {
                facts.status = Some("disconnecting…".into());
                model.set_session(Some(facts));
            }
            model.workbench_mut().mark_disconnected();
            Update {
                render: true,
                effect: Some(Effect::DisconnectSession {
                    request_token: token,
                    session_id_hex,
                }),
            }
        }
        ActionId::Cancel if model.screen() == Screen::Editor => {
            model.set_screen(Screen::Connections);
            model.set_action(ActionId::Open);
            Update::render()
        }
        ActionId::Quit => Update::with_effect(Effect::Exit),
        ActionId::NextDatabase if model.screen() == Screen::Workbench => {
            switch_workbench_database(model)
        }
        ActionId::NextTab if model.screen() == Screen::Workbench => {
            model.workbench_mut().select_next_tab();
            Update::render()
        }
        ActionId::PinTab if model.screen() == Screen::Workbench => {
            model.workbench_mut().promote_active_tab();
            Update::render()
        }
        ActionId::CloseTab if model.screen() == Screen::Workbench => {
            use crate::model::workbench::CloseTabOutcome;
            match model.workbench_mut().close_active_tab() {
                CloseTabOutcome::NeedsConfirm { title, index } => {
                    model.set_confirm(Some(ConfirmDialog::CloseDirtyTab { title, index }));
                    model.set_action(ActionId::Submit);
                    Update::render()
                }
                CloseTabOutcome::Closed | CloseTabOutcome::Empty => Update::render(),
            }
        }
        ActionId::NewSql if model.screen() == Screen::Workbench => {
            model.workbench_mut().open_sql_tab();
            Update::render()
        }
        ActionId::RunSql if model.screen() == Screen::Workbench => run_sql_or_bind_params(model),
        ActionId::RunScript if model.screen() == Screen::Workbench => run_script_entire_buffer(model),
        ActionId::CancelQuery if model.screen() == Screen::Workbench => {
            let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
                return Update::unchanged();
            };
            let token = model.mint_request_token();
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_cancel_requested();
            }
            Update {
                render: true,
                effect: Some(Effect::CancelQuery {
                    request_token: token,
                    session_id_hex,
                }),
            }
        }
        ActionId::Inspect if model.screen() == Screen::Workbench => {
            model.workbench_mut().inspect_cursor();
            Update::render()
        }
        ActionId::Complete if model.screen() == Screen::Workbench => {
            if model.workbench().completion.is_some() {
                let _ = model.workbench_mut().commit_completion(None);
            } else if model.workbench().active_editor().is_some() {
                model.workbench_mut().open_completion();
            } else {
                return Update::unchanged();
            }
            Update::render()
        }
        ActionId::History if model.screen() == Screen::Workbench => {
            if model.workbench().history.is_open() {
                model.workbench_mut().history = crate::model::history::HistoryPanel::Closed;
                return Update::render();
            }
            let token = model.mint_request_token();
            model.workbench_mut().history =
                crate::model::history::HistoryPanel::Loading { request_token: token };
            Update {
                render: true,
                effect: Some(Effect::LoadHistory {
                    request_token: token,
                    search: None,
                    limit: 50,
                }),
            }
        }
        ActionId::RestoreHistory if model.screen() == Screen::Workbench => {
            let text = model
                .workbench()
                .history
                .selected_entry()
                .map(|e| e.statement_preview.clone())
                .filter(|s| !s.is_empty() && s != "(no text)");
            let Some(text) = text else {
                return Update::unchanged();
            };
            // Ensure a SQL tab exists, then restore text without auto-executing.
            if model.workbench().active_editor().is_none() {
                model.workbench_mut().open_sql_tab();
            }
            let selected = model.workbench().selected_tab;
            if let Some(editor) = model.workbench_mut().active_editor_mut() {
                editor.set_text(text);
            }
            if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
                tab.dirty = true;
            }
            model.workbench_mut().history = crate::model::history::HistoryPanel::Closed;
            Update::render()
        }
        ActionId::SavedQueries if model.screen() == Screen::Workbench => {
            if model.workbench().saved_queries.is_open() {
                model.workbench_mut().saved_queries =
                    crate::model::saved_query::SavedQueryPanel::Closed;
                return Update::render();
            }
            let token = model.mint_request_token();
            let engine_label = model.workbench().engine_kind.clone();
            model.workbench_mut().saved_queries =
                crate::model::saved_query::SavedQueryPanel::Loading {
                    request_token: token,
                };
            Update {
                render: true,
                effect: Some(Effect::ListNamedQueries {
                    request_token: token,
                    engine_label,
                }),
            }
        }
        ActionId::SaveQuery if model.screen() == Screen::Workbench => {
            let statement = model
                .workbench()
                .active_editor()
                .map(|e| e.text().to_owned())
                .unwrap_or_default();
            if statement.trim().is_empty() {
                return Update::unchanged();
            }
            let name = model
                .workbench()
                .active_tab()
                .map(|t| t.title.clone())
                .unwrap_or_else(|| "query".into());
            let engine_label = model.workbench().engine_kind.clone();
            let token = model.mint_request_token();
            Update {
                render: true,
                effect: Some(Effect::SaveNamedQuery {
                    request_token: token,
                    name,
                    engine_label,
                    statement,
                }),
            }
        }
        ActionId::LoadQuery if model.screen() == Screen::Workbench => {
            let query_id = model
                .workbench()
                .saved_queries
                .selected()
                .map(|q| q.query_id);
            let Some(query_id) = query_id else {
                return Update::unchanged();
            };
            let token = model.mint_request_token();
            Update {
                render: true,
                effect: Some(Effect::LoadNamedQuery {
                    request_token: token,
                    query_id,
                }),
            }
        }
        ActionId::SaveFile if model.screen() == Screen::Workbench => {
            let text = model
                .workbench()
                .active_editor()
                .map(|e| e.text().to_owned())
                .unwrap_or_default();
            let path = model
                .workbench()
                .active_tab()
                .and_then(|t| t.bound_file.as_ref().map(|f| f.path.clone()))
                .or_else(|| {
                    model
                        .workbench()
                        .active_tab()
                        .map(|t| format!("{}.sql", t.title.replace(' ', "_")))
                });
            let Some(path) = path else {
                return Update::unchanged();
            };
            let token = model.mint_request_token();
            Update {
                render: true,
                effect: Some(Effect::SaveSqlFile {
                    request_token: token,
                    path,
                    text,
                }),
            }
        }
        ActionId::SaveIntent if model.screen() == Screen::Workbench => {
            let Some(profile_id_hex) = model.workbench().profile_id_hex.clone() else {
                return Update::unchanged();
            };
            let intent_json = model.workbench().intent_json();
            let token = model.mint_request_token();
            Update {
                render: true,
                effect: Some(Effect::SaveSessionIntent {
                    request_token: token,
                    profile_id_hex,
                    intent_json,
                }),
            }
        }
        ActionId::CopyCsv if model.screen() == Screen::Workbench => {
            copy_grid(model, crate::model::copy_format::CopyFormat::Csv)
        }
        ActionId::CopyTsv if model.screen() == Screen::Workbench => {
            copy_grid(model, crate::model::copy_format::CopyFormat::Tsv)
        }
        ActionId::CopyJson if model.screen() == Screen::Workbench => {
            copy_grid(model, crate::model::copy_format::CopyFormat::Json)
        }
        ActionId::CopyMarkdown if model.screen() == Screen::Workbench => {
            copy_grid(model, crate::model::copy_format::CopyFormat::Markdown)
        }
        ActionId::CopySqlInsert if model.screen() == Screen::Workbench => {
            copy_grid(model, crate::model::copy_format::CopyFormat::SqlInsert)
        }
        ActionId::CopySqlUpdate if model.screen() == Screen::Workbench => {
            copy_grid(model, crate::model::copy_format::CopyFormat::SqlUpdate)
        }
        ActionId::CopyCell if model.screen() == Screen::Workbench => copy_cursor_cell(model, false),
        ActionId::CopyCellHex if model.screen() == Screen::Workbench => {
            copy_cursor_cell(model, true)
        }
        ActionId::CopyRow if model.screen() == Screen::Workbench => copy_cursor_row(model),
        ActionId::CycleSort if model.screen() == Screen::Workbench => {
            let col = model
                .workbench()
                .active_grid()
                .and_then(|g| g.columns.get(g.cursor_col).cloned());
            let Some(col) = col else {
                return Update::unchanged();
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.cycle_sort_column(&col);
            }
            rebrowse_active_table(model)
        }
        ActionId::AddFilter if model.screen() == Screen::Workbench => {
            let (col, value) = {
                let Some(grid) = model.workbench().active_grid() else {
                    return Update::unchanged();
                };
                let col = grid.columns.get(grid.cursor_col).cloned();
                let value = grid.cell_at(grid.cursor_row, grid.cursor_col).text.clone();
                (col, value)
            };
            let Some(col) = col else {
                return Update::unchanged();
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                let is_null = value.is_empty()
                    || matches!(
                        grid.cell_at(grid.cursor_row, grid.cursor_col).distinction,
                        crate::model::grid::CellDistinction::Null
                    );
                if is_null {
                    grid.add_filter_chip(col, "isnull", None);
                } else {
                    grid.add_filter_chip(col, "eq", Some(value));
                }
            }
            rebrowse_active_table(model)
        }
        ActionId::SaveFilter if model.screen() == Screen::Workbench => {
            if model.workbench().profile_id_hex.is_none() {
                return Update::unchanged();
            }
            let (schema, table) = {
                let Some(grid) = model.workbench().active_grid() else {
                    return Update::unchanged();
                };
                (grid.base_schema.clone(), grid.base_table.clone())
            };
            let (Some(schema), Some(table)) = (schema, table) else {
                return Update::unchanged();
            };
            model.set_confirm(Some(ConfirmDialog::SaveFilter {
                schema,
                table,
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::ApplyFilter if model.screen() == Screen::Workbench => {
            let (schema, table) = {
                let Some(grid) = model.workbench().active_grid() else {
                    return Update::unchanged();
                };
                (grid.base_schema.clone(), grid.base_table.clone())
            };
            let (Some(schema), Some(table)) = (schema, table) else {
                return Update::unchanged();
            };
            let known_names = model
                .workbench()
                .filter_library
                .names_for_table(&schema, &table);
            model.set_confirm(Some(ConfirmDialog::ApplyFilter {
                schema,
                table,
                known_names,
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::ClearFilters if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.clear_server_controls();
            }
            rebrowse_active_table(model)
        }
        ActionId::ToggleColumn if model.screen() == Screen::Workbench => {
            let col = model
                .workbench()
                .active_grid()
                .and_then(|g| g.columns.get(g.cursor_col).cloned());
            let Some(col) = col else {
                return Update::unchanged();
            };
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                let _ = grid.toggle_column_visible(&col);
            }
            Update::render()
        }
        ActionId::ResetColumns if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.reset_column_layout();
            }
            Update::render()
        }
        ActionId::MoveColumnLeft if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if grid.move_cursor_column(-1) {
                    return Update::render();
                }
            }
            Update::unchanged()
        }
        ActionId::MoveColumnRight if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if grid.move_cursor_column(1) {
                    return Update::render();
                }
            }
            Update::unchanged()
        }
        ActionId::NarrowColumn if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if grid.adjust_cursor_column_width(-2) {
                    return Update::render();
                }
            }
            Update::unchanged()
        }
        ActionId::WidenColumn if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if grid.adjust_cursor_column_width(2) {
                    return Update::render();
                }
            }
            Update::unchanged()
        }
        ActionId::FitColumn if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if grid.fit_cursor_column() {
                    return Update::render();
                }
            }
            Update::unchanged()
        }
        ActionId::FitAllColumns if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if grid.fit_all_visible_columns() {
                    return Update::render();
                }
            }
            Update::unchanged()
        }
        ActionId::UndoStaged if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if grid.drafts.undo() {
                    let empty = grid.drafts.is_empty();
                    if empty {
                        model.workbench_mut().mark_active_dirty(false);
                    }
                    return Update::render();
                }
            }
            Update::unchanged()
        }
        ActionId::DiscardStaged if model.screen() == Screen::Workbench => {
            let had_redis = !model.workbench().redis_staged.is_empty();
            if had_redis {
                model.workbench_mut().redis_staged.clear();
                model.workbench_mut().mutation_review = None;
                model.workbench_mut().pending_review_token_hex = None;
                model.workbench_mut().pending_review_expires_at_ms = None;
            }
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if !grid.drafts.is_empty() || grid.cell_edit.is_some() || had_redis {
                    grid.drafts.discard_all();
                    grid.cell_edit = None;
                    model.workbench_mut().mutation_review = None;
                    model.workbench_mut().mark_active_dirty(false);
                    return Update::render();
                }
            } else if had_redis {
                model.workbench_mut().mark_active_dirty(false);
                return Update::render();
            }
            Update::unchanged()
        }
        ActionId::StageRedisAdd if model.screen() == Screen::Workbench => {
            open_redis_stage_confirm(model, true)
        }
        ActionId::StageRedisRemove if model.screen() == Screen::Workbench => {
            open_redis_stage_confirm(model, false)
        }
        ActionId::RedisSubscribe if model.screen() == Screen::Workbench => {
            model.set_confirm(Some(ConfirmDialog::RedisSubscribe {
                pattern: false,
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::RedisPSubscribe if model.screen() == Screen::Workbench => {
            model.set_confirm(Some(ConfirmDialog::RedisSubscribe {
                pattern: true,
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::RedisCollectionMore if model.screen() == Screen::Workbench => {
            let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
                return Update::unchanged();
            };
            let Some(skip) = model.workbench().redis_collection_skip else {
                if let Some(grid) = model.workbench_mut().active_grid_mut() {
                    grid.error_label = Some("no more collection pages".into());
                }
                return Update::render();
            };
            let Some((_, key, _)) = model.workbench().redis_stage_target.clone() else {
                return Update::unchanged();
            };
            let token = model.mint_request_token();
            let context_revision = model.workbench().context_revision;
            Update {
                render: true,
                effect: Some(Effect::OpenRedisKey {
                    request_token: token,
                    session_id_hex,
                    context_revision,
                    key,
                    collection_skip: skip,
                }),
            }
        }
        ActionId::Cancel
            if model.screen() == Screen::Workbench
                && model
                    .workbench()
                    .active_grid()
                    .is_some_and(|g| g.cell_edit.is_some()) =>
        {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.cancel_cell_edit();
            }
            model.workbench_mut().mutation_review = None;
            Update::render()
        }
        ActionId::ReviewMutations if model.screen() == Screen::Workbench => {
            register_staged_review(model)
        }
        ActionId::EditCell if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if grid.begin_cell_edit() {
                    return Update::render();
                }
            }
            Update::unchanged()
        }
        ActionId::ToggleBool if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if let Some(edit) = grid.cell_edit.as_mut() {
                    if edit.toggle_boolean() {
                        return Update::render();
                    }
                }
            }
            Update::unchanged()
        }
        ActionId::SetNull if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if let Some(edit) = grid.cell_edit.as_mut() {
                    edit.set_null();
                    return Update::render();
                }
            }
            Update::unchanged()
        }
        ActionId::SetToday if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if let Some(edit) = grid.cell_edit.as_mut() {
                    if edit.set_today() {
                        return Update::render();
                    }
                }
            }
            Update::unchanged()
        }
        ActionId::SetNow if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if let Some(edit) = grid.cell_edit.as_mut() {
                    if edit.set_now() {
                        return Update::render();
                    }
                }
            }
            Update::unchanged()
        }
        ActionId::IncNumber if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if let Some(edit) = grid.cell_edit.as_mut() {
                    if edit.step_number(1) {
                        return Update::render();
                    }
                }
            }
            Update::unchanged()
        }
        ActionId::DecNumber if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if let Some(edit) = grid.cell_edit.as_mut() {
                    if edit.step_number(-1) {
                        return Update::render();
                    }
                }
            }
            Update::unchanged()
        }
        ActionId::FormatJson if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if let Some(edit) = grid.cell_edit.as_mut() {
                    if edit.format_structured() {
                        return Update::render();
                    }
                }
            }
            Update::unchanged()
        }
        ActionId::CompactJson if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if let Some(edit) = grid.cell_edit.as_mut() {
                    if edit.compact_structured() {
                        return Update::render();
                    }
                }
            }
            Update::unchanged()
        }
        ActionId::DeleteRow if model.screen() == Screen::Workbench => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if grid.stage_delete_cursor_row() {
                    model.workbench_mut().mark_active_dirty(true);
                    return Update::render();
                }
            }
            Update::unchanged()
        }
        ActionId::ApplyMutations if model.screen() == Screen::Workbench => {
            apply_staged_mutations(model)
        }
        ActionId::FollowForeignKey if model.screen() == Screen::Workbench => {
            follow_foreign_key(model)
        }
        ActionId::ShowStructure if model.screen() == Screen::Workbench => show_structure(model),
        ActionId::CopyStructureDdl if model.screen() == Screen::Workbench => {
            copy_structure_ddl(model)
        }
        ActionId::TruncateTable if model.screen() == Screen::Workbench => {
            let Some(grid) = model.workbench().active_grid() else {
                return Update::unchanged();
            };
            let (Some(schema), Some(table)) =
                (grid.base_schema.clone(), grid.base_table.clone())
            else {
                return Update::unchanged();
            };
            model.set_confirm(Some(ConfirmDialog::TruncateTable {
                schema,
                table,
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::DropTable if model.screen() == Screen::Workbench => {
            let Some(grid) = model.workbench().active_grid() else {
                return Update::unchanged();
            };
            let (Some(schema), Some(table)) =
                (grid.base_schema.clone(), grid.base_table.clone())
            else {
                return Update::unchanged();
            };
            model.set_confirm(Some(ConfirmDialog::DropTable {
                schema,
                table,
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::VacuumTable if model.screen() == Screen::Workbench => {
            let Some(grid) = model.workbench().active_grid() else {
                return Update::unchanged();
            };
            let (Some(schema), Some(table)) =
                (grid.base_schema.clone(), grid.base_table.clone())
            else {
                return Update::unchanged();
            };
            model.set_confirm(Some(ConfirmDialog::VacuumTable {
                schema,
                table,
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::AnalyzeTable if model.screen() == Screen::Workbench => {
            let Some(grid) = model.workbench().active_grid() else {
                return Update::unchanged();
            };
            let (Some(schema), Some(table)) =
                (grid.base_schema.clone(), grid.base_table.clone())
            else {
                return Update::unchanged();
            };
            model.set_confirm(Some(ConfirmDialog::AnalyzeTable {
                schema,
                table,
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::OptimizeTable if model.screen() == Screen::Workbench => {
            let Some(grid) = model.workbench().active_grid() else {
                return Update::unchanged();
            };
            let (Some(schema), Some(table)) =
                (grid.base_schema.clone(), grid.base_table.clone())
            else {
                return Update::unchanged();
            };
            model.set_confirm(Some(ConfirmDialog::OptimizeTable {
                schema,
                table,
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::RenameTable if model.screen() == Screen::Workbench => {
            let Some(grid) = model.workbench().active_grid() else {
                return Update::unchanged();
            };
            let (Some(schema), Some(table)) =
                (grid.base_schema.clone(), grid.base_table.clone())
            else {
                return Update::unchanged();
            };
            model.set_confirm(Some(ConfirmDialog::RenameTable {
                schema,
                table,
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::DdlAddColumn if model.screen() == Screen::Workbench => open_ddl_review(
            model,
            "add_column",
            "ADD COLUMN on {schema}.{table} (paste: col_name type)",
        ),
        ActionId::DdlCreateIndex if model.screen() == Screen::Workbench => open_ddl_review(
            model,
            "create_index",
            "CREATE INDEX on {schema}.{table} (paste: index_name column)",
        ),
        ActionId::DdlDropColumn if model.screen() == Screen::Workbench => open_ddl_review(
            model,
            "drop_column",
            "DROP COLUMN on {schema}.{table} (paste: column_name)",
        ),
        ActionId::DdlDropIndex if model.screen() == Screen::Workbench => open_ddl_review(
            model,
            "drop_index",
            "DROP INDEX on {schema}.{table} (paste: index_name)",
        ),
        ActionId::DdlAddConstraint if model.screen() == Screen::Workbench => open_ddl_review(
            model,
            "add_constraint",
            "ADD CONSTRAINT on {schema}.{table} (paste: name UNIQUE (col))",
        ),
        ActionId::DdlDropConstraint if model.screen() == Screen::Workbench => open_ddl_review(
            model,
            "drop_constraint",
            "DROP CONSTRAINT on {schema}.{table} (paste: name)",
        ),
        ActionId::ShowActivity if model.screen() == Screen::Workbench => {
            let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
                return Update::unchanged();
            };
            let token = model.mint_request_token();
            let context_revision = model.workbench().context_revision;
            Update {
                render: true,
                effect: Some(Effect::LoadActivity {
                    request_token: token,
                    session_id_hex,
                    context_revision,
                }),
            }
        }
        ActionId::ShowRoles if model.screen() == Screen::Workbench => {
            let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
                return Update::unchanged();
            };
            let (schema, table) = model
                .workbench()
                .active_grid()
                .map(|g| (g.base_schema.clone(), g.base_table.clone()))
                .unwrap_or((None, None));
            let token = model.mint_request_token();
            let context_revision = model.workbench().context_revision;
            Update {
                render: true,
                effect: Some(Effect::LoadRoles {
                    request_token: token,
                    session_id_hex,
                    context_revision,
                    schema,
                    table,
                }),
            }
        }
        ActionId::ScanRedisKeys if model.screen() == Screen::Workbench => {
            let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
                return Update::unchanged();
            };
            if !model
                .workbench()
                .engine_kind
                .eq_ignore_ascii_case("Redis")
            {
                return Update::unchanged();
            }
            // Catalog tree filter becomes SCAN MATCH (empty → all keys).
            let pattern = match &model.workbench().catalog {
                CatalogModel::Loaded { filter, .. } if !filter.trim().is_empty() => {
                    filter.trim().to_owned()
                }
                _ => "*".into(),
            };
            let token = model.mint_request_token();
            let context_revision = model.workbench().context_revision;
            Update {
                render: true,
                effect: Some(Effect::ScanRedisKeys {
                    request_token: token,
                    session_id_hex,
                    context_revision,
                    pattern,
                    count: 100,
                }),
            }
        }
        ActionId::RedisInfo if model.screen() == Screen::Workbench => {
            let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
                return Update::unchanged();
            };
            let token = model.mint_request_token();
            let context_revision = model.workbench().context_revision;
            Update {
                render: true,
                effect: Some(Effect::LoadRedisInfo {
                    request_token: token,
                    session_id_hex,
                    context_revision,
                }),
            }
        }
        ActionId::ExportCsv if model.screen() == Screen::Workbench => {
            export_loaded_result(model, "csv", "export.csv")
        }
        ActionId::ExportJson if model.screen() == Screen::Workbench => {
            export_loaded_result(model, "json", "export.json")
        }
        ActionId::ExportTsv if model.screen() == Screen::Workbench => {
            export_loaded_result(model, "tsv", "export.tsv")
        }
        ActionId::ExportStreamCsv if model.screen() == Screen::Workbench => {
            export_stream_query(model, "csv", "export-stream.csv")
        }
        ActionId::ExportStreamJson if model.screen() == Screen::Workbench => {
            export_stream_query(model, "json", "export-stream.json")
        }
        ActionId::ExportStreamTsv if model.screen() == Screen::Workbench => {
            export_stream_query(model, "tsv", "export-stream.tsv")
        }
        ActionId::ImportCsv if model.screen() == Screen::Workbench => import_csv_apply(model),
        ActionId::Explain if model.screen() == Screen::Workbench => explain_active_sql(model),
        ActionId::QuickSwitch
            if matches!(
                model.screen(),
                Screen::Workbench | Screen::Connections | Screen::ConnectionPicker
            ) =>
        {
            model.set_confirm(Some(ConfirmDialog::QuickSwitch {
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::FindReplace if model.screen() == Screen::Workbench => {
            if model.workbench().active_editor().is_none() {
                return Update::unchanged();
            }
            model.set_confirm(Some(ConfirmDialog::FindReplace {
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::FormatSql if model.screen() == Screen::Workbench => {
            let Some(ed) = model.workbench_mut().active_editor_mut() else {
                return Update::unchanged();
            };
            let dialect = ed.dialect();
            let formatted = tablerock_core::format_sql(ed.text(), dialect);
            ed.set_text(formatted);
            Update::render()
        }
        ActionId::PgDump if model.screen() == Screen::Workbench => {
            if !model
                .session()
                .map(|s| s.engine_label.eq_ignore_ascii_case("PostgreSQL"))
                .unwrap_or(false)
            {
                return Update::unchanged();
            }
            model.set_confirm(Some(ConfirmDialog::PgTool {
                kind: "dump".into(),
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::PgRestore if model.screen() == Screen::Workbench => {
            if !model
                .session()
                .map(|s| s.engine_label.eq_ignore_ascii_case("PostgreSQL"))
                .unwrap_or(false)
            {
                return Update::unchanged();
            }
            model.set_confirm(Some(ConfirmDialog::PgTool {
                kind: "restore".into(),
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::CancelBackend if model.screen() == Screen::Workbench => {
            model.set_confirm(Some(ConfirmDialog::CancelBackend {
                pid: String::new(),
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::TerminateBackend if model.screen() == Screen::Workbench => {
            model.set_confirm(Some(ConfirmDialog::TerminateBackend {
                pid: String::new(),
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::KillMutation if model.screen() == Screen::Workbench => {
            // ClickHouse only; require an active base table for targeting.
            let is_ch = model
                .session()
                .map(|s| s.engine_label.eq_ignore_ascii_case("ClickHouse"))
                .unwrap_or(false);
            if !is_ch {
                return Update::unchanged();
            }
            let Some(grid) = model.workbench().active_grid() else {
                return Update::unchanged();
            };
            let (Some(database), Some(table)) =
                (grid.base_schema.clone(), grid.base_table.clone())
            else {
                return Update::unchanged();
            };
            model.set_confirm(Some(ConfirmDialog::KillMutation {
                database,
                table,
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        ActionId::SaveColumns if model.screen() == Screen::Workbench => {
            let profile_id_hex = model.workbench().profile_id_hex.clone();
            let database = model.workbench().context.database.clone();
            let (schema, table, layout_json) = {
                let Some(grid) = model.workbench().active_grid() else {
                    return Update::unchanged();
                };
                (
                    grid.base_schema.clone(),
                    grid.base_table.clone(),
                    grid.layout_json(),
                )
            };
            let (Some(profile_id_hex), Some(schema), Some(table)) =
                (profile_id_hex, schema, table)
            else {
                return Update::unchanged();
            };
            let token = model.mint_request_token();
            Update {
                render: true,
                effect: Some(Effect::SaveColumnLayout {
                    request_token: token,
                    profile_id_hex,
                    database,
                    schema,
                    table,
                    layout_json,
                }),
            }
        }
        ActionId::Cancel if model.screen() == Screen::Workbench && model.workbench().completion.is_some() =>
        {
            model.workbench_mut().dismiss_completion();
            Update::render()
        }
        ActionId::Save
        | ActionId::Test
        | ActionId::Connect
        | ActionId::Disconnect
        | ActionId::Remove
        | ActionId::RenameGroup
        | ActionId::Reconnect
        | ActionId::SessionHealth
        | ActionId::NextDatabase
        | ActionId::NextTab
        | ActionId::CloseTab
        | ActionId::PinTab
        | ActionId::NewSql
        | ActionId::RunSql
        | ActionId::RunScript
        | ActionId::CancelQuery
        | ActionId::Inspect
        | ActionId::Complete
        | ActionId::History
        | ActionId::RestoreHistory
        | ActionId::SavedQueries
        | ActionId::SaveQuery
        | ActionId::LoadQuery
        | ActionId::SaveFile
        | ActionId::SaveIntent
        | ActionId::CopyCsv
        | ActionId::CopyTsv
        | ActionId::CopyJson
        | ActionId::CopyMarkdown
        | ActionId::CopySqlInsert
        | ActionId::CopySqlUpdate
        | ActionId::CopyCell
        | ActionId::CopyCellHex
        | ActionId::CopyRow
        | ActionId::CycleSort
        | ActionId::AddFilter
        | ActionId::SaveFilter
        | ActionId::ApplyFilter
        | ActionId::ClearFilters
        | ActionId::ToggleColumn
        | ActionId::ResetColumns
        | ActionId::SaveColumns
        | ActionId::MoveColumnLeft
        | ActionId::MoveColumnRight
        | ActionId::NarrowColumn
        | ActionId::WidenColumn
        | ActionId::FitColumn
        | ActionId::FitAllColumns
        | ActionId::UndoStaged
        | ActionId::DiscardStaged
        | ActionId::ReviewMutations
        | ActionId::EditCell
        | ActionId::ToggleBool
        | ActionId::SetNull
        | ActionId::SetToday
        | ActionId::SetNow
        | ActionId::IncNumber
        | ActionId::DecNumber
        | ActionId::FormatJson
        | ActionId::CompactJson
        | ActionId::DeleteRow
        | ActionId::ApplyMutations
        | ActionId::FollowForeignKey
        | ActionId::ShowStructure
        | ActionId::CopyStructureDdl
        | ActionId::TruncateTable
        | ActionId::DropTable
        | ActionId::VacuumTable
        | ActionId::AnalyzeTable
        | ActionId::OptimizeTable
        | ActionId::DdlAddColumn
        | ActionId::DdlCreateIndex
        | ActionId::DdlDropColumn
        | ActionId::DdlDropIndex
        | ActionId::DdlAddConstraint
        | ActionId::DdlDropConstraint
        | ActionId::RenameTable
        | ActionId::ShowActivity
        | ActionId::ShowRoles
        | ActionId::CancelBackend
        | ActionId::TerminateBackend
        | ActionId::KillMutation
        | ActionId::ScanRedisKeys
        | ActionId::RedisInfo
        | ActionId::StageRedisAdd
        | ActionId::StageRedisRemove
        | ActionId::RedisCollectionMore
        | ActionId::RedisSubscribe
        | ActionId::RedisPSubscribe
        | ActionId::ExportCsv
        | ActionId::ExportJson
        | ActionId::ExportTsv
        | ActionId::ExportStreamCsv
        | ActionId::ExportStreamJson
        | ActionId::ExportStreamTsv
        | ActionId::ImportCsv
        | ActionId::PgDump
        | ActionId::PgRestore
        | ActionId::ImportUrl
        | ActionId::OpenExternalUrl
        | ActionId::Explain
        | ActionId::QuickSwitch
        | ActionId::FindReplace
        | ActionId::FormatSql
        | ActionId::Submit
        | ActionId::Cancel => Update::unchanged(),
    }
}

/// Ranked quick-switch hit (lower score = better).
#[derive(Debug, Clone)]
enum QuickHit {
    Tab { index: usize },
    Profile { id_hex: String },
    SavedQuery { query_id: i64 },
}

fn rank_label(needle: &str, label: &str) -> Option<u32> {
    if needle.is_empty() {
        return Some(100);
    }
    let n = needle.to_ascii_lowercase();
    let l = label.to_ascii_lowercase();
    if l == n {
        Some(0)
    } else if l.starts_with(&n) {
        Some(1)
    } else if l.contains(&n) {
        Some(2 + (l.find(&n).unwrap_or(0) as u32).min(50))
    } else {
        None
    }
}

fn apply_quick_switch(model: &mut Model, needle: &str) -> Update {
    use crate::model::profiles::ProfileListState;
    use crate::model::saved_query::SavedQueryPanel;

    // Collect ranked candidates by screen.
    let mut hits: Vec<(u32, QuickHit)> = Vec::new();

    match model.screen() {
        Screen::Workbench => {
            for (i, tab) in model.workbench().tabs.iter().enumerate() {
                let label = format!("t:{i}:{}", tab.title);
                if let Some(score) = rank_label(needle, &tab.title)
                    .or_else(|| rank_label(needle, &label))
                    .or_else(|| {
                        if needle.parse::<usize>().ok() == Some(i + 1) {
                            Some(0)
                        } else {
                            None
                        }
                    })
                {
                    hits.push((score, QuickHit::Tab { index: i }));
                }
            }
            if let SavedQueryPanel::Open { entries, .. } = &model.workbench().saved_queries {
                for q in entries {
                    let label = format!("q:{}", q.name);
                    if let Some(score) = rank_label(needle, &q.name)
                        .or_else(|| rank_label(needle, &label))
                        .or_else(|| rank_label(needle, &q.statement_preview))
                    {
                        hits.push((
                            score.saturating_add(10), // prefer tabs slightly
                            QuickHit::SavedQuery {
                                query_id: q.query_id,
                            },
                        ));
                    }
                }
            }
        }
        Screen::Connections | Screen::ConnectionPicker => {
            if let ProfileListState::Loaded { rows, .. } = model.profiles() {
                for row in rows {
                    let labels = [
                        row.name.as_str(),
                        row.target_summary.as_str(),
                        row.engine_label.as_str(),
                    ];
                    let mut best = None;
                    for lab in labels {
                        if let Some(s) = rank_label(needle, lab) {
                            best = Some(best.map_or(s, |b: u32| b.min(s)));
                        }
                    }
                    if let Some(score) = best {
                        hits.push((
                            score,
                            QuickHit::Profile {
                                id_hex: row.id_hex.clone(),
                            },
                        ));
                    }
                }
            }
        }
        Screen::Editor => {}
    }

    hits.sort_by_key(|(score, _)| *score);
    model.set_confirm(None);

    let Some((_, hit)) = hits.into_iter().next() else {
        if let Some(g) = model.workbench_mut().active_grid_mut() {
            g.mark_failed(format!("no switch match for '{needle}'"));
        } else if matches!(
            model.screen(),
            Screen::Connections | Screen::ConnectionPicker
        ) {
            // Surface via profiles failure-style status without dropping list.
            if let ProfileListState::Loaded { .. } = model.profiles() {
                // keep loaded; no-op failure surface via editor if present
            }
        }
        return Update::render();
    };

    match hit {
        QuickHit::Tab { index } => {
            model.workbench_mut().selected_tab = index;
            Update::render()
        }
        QuickHit::Profile { id_hex } => {
            if let ProfileListState::Loaded { selected_id, .. } = model.profiles_mut() {
                *selected_id = Some(id_hex);
            }
            Update::render()
        }
        QuickHit::SavedQuery { query_id } => {
            let token = model.mint_request_token();
            Update {
                render: true,
                effect: Some(Effect::LoadNamedQuery {
                    request_token: token,
                    query_id,
                }),
            }
        }
    }
}

/// Run active SQL, expanding `:name` parameters; prompt when unbound.
fn run_sql_or_bind_params(model: &mut Model) -> Update {
    let Some(session) = model.session() else {
        return Update::unchanged();
    };
    let session_id_hex = session.session_id_hex.clone();
    // Redis command editor: sequential pipeline (no MULTI/EXEC).
    if model
        .workbench()
        .engine_kind
        .eq_ignore_ascii_case("Redis")
    {
        return run_redis_pipeline(model, session_id_hex);
    }
    // Multi-statement script: only when an explicit selection covers ≥2 spans
    // (default Run stays "current statement under cursor" / selection slice).
    {
        let Some(ed) = model.workbench().active_editor() else {
            return Update::unchanged();
        };
        if let Some((a, b)) = ed.selection() {
            let (start, end) = if a <= b { (a, b) } else { (b, a) };
            let covered = ed
                .spans()
                .iter()
                .filter(|s| s.start < end && s.end > start)
                .count();
            if covered >= 2 {
                return run_sql_script(model, session_id_hex);
            }
        }
    }
    let statement = model
        .workbench()
        .active_editor()
        .and_then(|ed| ed.run_text())
        .unwrap_or_default();
    if statement.trim().is_empty() {
        return Update::unchanged();
    }
    match prepare_executable_sql(&statement) {
        Ok((sql, parameters)) => emit_execute_sql(model, session_id_hex, sql, parameters),
        Err(missing) if !missing.is_empty() => {
            model.set_confirm(Some(ConfirmDialog::BindParams {
                names: missing,
                statement,
                confirm_buffer: String::new(),
            }));
            model.set_action(ActionId::Submit);
            Update::render()
        }
        Err(_) => Update::unchanged(),
    }
}

/// Explicit RunScript: entire editor buffer as ordered multi-statement script.
/// Redis is redirected to the sequential command pipeline (same as Run).
fn run_script_entire_buffer(model: &mut Model) -> Update {
    let Some(session) = model.session() else {
        return Update::unchanged();
    };
    let session_id_hex = session.session_id_hex.clone();
    if model
        .workbench()
        .engine_kind
        .eq_ignore_ascii_case("Redis")
    {
        return run_redis_pipeline(model, session_id_hex);
    }
    let Some(ed) = model.workbench_mut().active_editor_mut() else {
        return Update::unchanged();
    };
    // Force full-buffer coverage regardless of cursor/selection.
    let len = ed.text().len();
    ed.set_selection(0, len);
    run_sql_script(model, session_id_hex)
}

/// Redis command editor: tokenize lines, deny mixed blocking, run sequential pipeline.
///
/// A **single** BLPOP/BRPOP with a key uses the disposable-connection path
/// (engine `blocking_pop`). Mixed pipelines still deny blocking lines.
fn run_redis_pipeline(model: &mut Model, session_id_hex: String) -> Update {
    use crate::model::redis_command::{RedisCommandSafety, parse_command_line};
    let Some(ed) = model.workbench().active_editor() else {
        return Update::unchanged();
    };
    let text = ed
        .run_text()
        .unwrap_or_else(|| ed.text().to_owned());
    let mut commands: Vec<(String, Vec<String>)> = Vec::new();
    let mut isolated_blocking: Option<(String, String)> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parsed = parse_command_line(line);
        match parsed.safety {
            RedisCommandSafety::Empty => continue,
            RedisCommandSafety::BlockingDenied => {
                // Isolated path only for lone BLPOP/BRPOP + first key.
                if matches!(parsed.name.as_str(), "BLPOP" | "BRPOP")
                    && !parsed.args.is_empty()
                    && commands.is_empty()
                    && isolated_blocking.is_none()
                {
                    isolated_blocking = Some((parsed.name, parsed.args[0].clone()));
                    continue;
                }
                if let Some(grid) = model.workbench_mut().active_grid_mut() {
                    grid.mark_failed(format!(
                        "blocking command denied on shared pipeline: {}",
                        parsed.name
                    ));
                }
                return Update::render();
            }
            RedisCommandSafety::ReadOnly | RedisCommandSafety::MayWrite => {
                if isolated_blocking.is_some() {
                    // Blocking + later non-blocking → deny (no mixed isolated).
                    if let Some(grid) = model.workbench_mut().active_grid_mut() {
                        grid.mark_failed(
                            "blocking BLPOP/BRPOP must be alone for isolated connection",
                        );
                    }
                    return Update::render();
                }
                commands.push((parsed.name, parsed.args));
            }
        }
    }
    if let Some((_name, key)) = isolated_blocking {
        if !commands.is_empty() {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_failed(
                    "blocking BLPOP/BRPOP must be alone for isolated connection",
                );
            }
            return Update::render();
        }
        let token = model.mint_request_token();
        let context_revision = model.workbench().context_revision;
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.operation = GridOperationState::Running;
            grid.error_label = None;
        }
        let selected = model.workbench().selected_tab;
        if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
            tab.running = true;
        }
        return Update {
            render: true,
            effect: Some(Effect::RedisBlockingPop {
                request_token: token,
                session_id_hex,
                context_revision,
                key,
            }),
        };
    }
    if commands.is_empty() {
        return Update::unchanged();
    }
    if commands.len() > 64 {
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.mark_failed("redis pipeline exceeds 64 commands");
        }
        return Update::render();
    }
    let token = model.mint_request_token();
    let context_revision = model.workbench().context_revision;
    if let Some(grid) = model.workbench_mut().active_grid_mut() {
        grid.operation = GridOperationState::Running;
        grid.error_label = None;
    }
    let selected = model.workbench().selected_tab;
    if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
        tab.running = true;
    }
    Update {
        render: true,
        effect: Some(Effect::ExecuteRedisPipeline {
            request_token: token,
            session_id_hex,
            context_revision,
            commands,
        }),
    }
}

fn run_sql_script(model: &mut Model, session_id_hex: String) -> Update {
    use tablerock_core::statements;
    let Some(ed) = model.workbench().active_editor() else {
        return Update::unchanged();
    };
    let dialect = ed.dialect();
    let text = ed.text().to_owned();
    let (sel_start, sel_end) = match ed.selection() {
        Some((a, b)) if a <= b => (a, b),
        Some((a, b)) => (b, a),
        None => (0, text.len()),
    };
    let spans = statements(&text, dialect);
    let mut prepared = Vec::new();
    let mut all_params = Vec::new();
    for span in &spans {
        if span.end <= sel_start || span.start >= sel_end {
            continue;
        }
        let slice = text[span.start..span.end.min(text.len())]
            .trim()
            .trim_end_matches(';')
            .trim();
        if slice.is_empty() {
            continue;
        }
        match prepare_executable_sql(slice) {
            Ok((sql, params)) => {
                if all_params.is_empty() && !params.is_empty() {
                    all_params = params;
                }
                prepared.push(sql);
            }
            Err(missing) => {
                model.set_confirm(Some(ConfirmDialog::BindParams {
                    names: missing,
                    statement: text,
                    confirm_buffer: String::new(),
                }));
                model.set_action(ActionId::Submit);
                return Update::render();
            }
        }
    }
    if prepared.is_empty() {
        return Update::unchanged();
    }
    if prepared.len() == 1 {
        return emit_execute_sql(model, session_id_hex, prepared.remove(0), all_params);
    }
    let token = model.mint_request_token();
    let context_revision = model.workbench().context_revision;
    model.workbench_mut().result_sections = crate::model::result_sections::ResultSectionsModel {
        sections: prepared
            .iter()
            .enumerate()
            .map(|(i, _)| crate::model::result_sections::StatementSection {
                ordinal: (i + 1) as u32,
                command_tag: "stmt".into(),
                kind: crate::model::result_sections::StatementSectionKind::Pending,
                rows: None,
                elapsed_ms: None,
                error: None,
                pinned: false,
            })
            .collect(),
        selected: 0,
    };
    if let Some(grid) = model.workbench_mut().active_grid_mut() {
        grid.operation = GridOperationState::Running;
        grid.error_label = None;
    }
    let selected = model.workbench().selected_tab;
    if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
        tab.running = true;
    }
    Update {
        render: true,
        effect: Some(Effect::ExecuteSqlScript {
            request_token: token,
            session_id_hex,
            context_revision,
            statements: prepared,
            parameters: all_params,
        }),
    }
}

/// Rewrite named params. Ok = ready to execute; Err = missing binding names.
fn prepare_executable_sql(statement: &str) -> Result<(String, Vec<String>), Vec<String>> {
    use tablerock_core::{bind_named_values, parse_param_bindings, rewrite_named_params};
    let plan = match rewrite_named_params(statement) {
        Ok(p) => p,
        // Invalid rewrite: run original text unbound (no silent `:name` expand).
        Err(_) => return Ok((statement.to_owned(), Vec::new())),
    };
    if plan.names.is_empty() {
        return Ok((plan.sql, Vec::new()));
    }
    let empty = parse_param_bindings("");
    match bind_named_values(&plan, &empty) {
        Ok(vals) => Ok((plan.sql, vals)),
        Err(missing) => Err(missing),
    }
}

fn emit_execute_sql(
    model: &mut Model,
    session_id_hex: String,
    statement: String,
    parameters: Vec<String>,
) -> Update {
    let token = model.mint_request_token();
    let context_revision = model.workbench().context_revision;
    if let Some(grid) = model.workbench_mut().active_grid_mut() {
        grid.operation = GridOperationState::Running;
        grid.error_label = None;
    }
    let selected = model.workbench().selected_tab;
    if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
        tab.running = true;
    }
    Update {
        render: true,
        effect: Some(Effect::ExecuteSql {
            request_token: token,
            session_id_hex,
            context_revision,
            statement,
            parameters,
        }),
    }
}

/// Prefix the active editor statement with engine-appropriate EXPLAIN.
fn explain_active_sql(model: &mut Model) -> Update {
    let Some(session) = model.session() else {
        return Update::unchanged();
    };
    let engine = session.engine_label.clone();
    let session_id_hex = session.session_id_hex.clone();
    if engine.eq_ignore_ascii_case("Redis") {
        if let Some(g) = model.workbench_mut().active_grid_mut() {
            g.mark_failed("EXPLAIN is unsupported for Redis");
        }
        return Update::render();
    }
    let statement = model
        .workbench()
        .active_editor()
        .and_then(|ed| ed.run_text())
        .unwrap_or_default();
    let body = statement.trim();
    if body.is_empty() {
        if let Some(g) = model.workbench_mut().active_grid_mut() {
            g.mark_failed("EXPLAIN needs SQL in the active editor");
        }
        return Update::render();
    }
    // Fail closed: never wrap statements that already start with EXPLAIN.
    let lower = body.to_ascii_lowercase();
    let explained = if lower.starts_with("explain") {
        body.to_owned()
    } else if engine.eq_ignore_ascii_case("ClickHouse") {
        format!("EXPLAIN {body}")
    } else {
        // PostgreSQL: plan only (no ANALYZE — ANALYZE executes the query).
        format!("EXPLAIN (FORMAT TEXT) {body}")
    };
    emit_execute_sql(model, session_id_hex, explained, Vec::new())
}

fn export_stream_query(model: &mut Model, format: &str, default_path: &str) -> Update {
    let Some(session) = model.session() else {
        return Update::unchanged();
    };
    let session_id_hex = session.session_id_hex.clone();
    let statement = model
        .workbench()
        .active_editor()
        .and_then(|ed| ed.run_text())
        .unwrap_or_default();
    if statement.trim().is_empty() {
        if let Some(g) = model.workbench_mut().active_grid_mut() {
            g.mark_failed("stream export needs SQL in the active editor");
        }
        return Update::render();
    }
    let token = model.mint_request_token();
    let context_revision = model.workbench().context_revision;
    Update {
        render: true,
        effect: Some(Effect::ExportStreamQuery {
            request_token: token,
            session_id_hex,
            context_revision,
            statement,
            path: default_path.into(),
            format: format.into(),
        }),
    }
}

fn import_csv_apply(model: &mut Model) -> Update {
    let Some(session) = model.session() else {
        return Update::unchanged();
    };
    let session_id_hex = session.session_id_hex.clone();
    let database = model.workbench().context.database.clone();
    let (schema, table) = {
        let Some(grid) = model.workbench().active_grid() else {
            return Update::unchanged();
        };
        (grid.base_schema.clone(), grid.base_table.clone())
    };
    let (Some(schema), Some(table)) = (schema, table) else {
        if let Some(g) = model.workbench_mut().active_grid_mut() {
            g.mark_failed("import CSV needs an active base table");
        }
        return Update::render();
    };
    let token = model.mint_request_token();
    let context_revision = model.workbench().context_revision;
    Update {
        render: true,
        effect: Some(Effect::ImportCsvApply {
            request_token: token,
            session_id_hex,
            context_revision,
            database,
            schema,
            table,
            path: "import.csv".into(),
        }),
    }
}

fn export_loaded_result(model: &mut Model, format: &str, default_path: &str) -> Update {
    use crate::model::copy_format::{CopyFormat, CopyScope, format_copy};
    let Some(grid) = model.workbench().active_grid() else {
        return Update::unchanged();
    };
    let fmt = match format {
        "json" => CopyFormat::Json,
        "tsv" => CopyFormat::Tsv,
        _ => CopyFormat::Csv,
    };
    let body = match format_copy(grid, CopyScope::LoadedResult, fmt) {
        Ok(s) => s,
        Err(e) => {
            if let Some(g) = model.workbench_mut().active_grid_mut() {
                g.mark_failed(e.to_string());
            }
            return Update::render();
        }
    };
    let token = model.mint_request_token();
    Update {
        render: true,
        effect: Some(Effect::ExportResult {
            request_token: token,
            path: default_path.into(),
            format: format.into(),
            body,
        }),
    }
}

fn follow_foreign_key(model: &mut Model) -> Update {
    let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
        return Update::unchanged();
    };
    let context_revision = model.workbench().context_revision;
    let (schema, table, local_column, row_cells) = {
        let Some(grid) = model.workbench().active_grid() else {
            return Update::unchanged();
        };
        let (schema, table) = match (grid.base_schema.clone(), grid.base_table.clone()) {
            (Some(s), Some(t)) => (s, t),
            _ => return Update::unchanged(),
        };
        if grid.columns.is_empty() {
            return Update::unchanged();
        }
        let col_idx = grid.cursor_col.min(grid.columns.len().saturating_sub(1));
        let local_column = grid.columns[col_idx].clone();
        // Snapshot full row so multi-column FKs can gather every key part.
        let row = grid.cursor_row;
        let row_cells: Vec<(String, String)> = grid
            .columns
            .iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), grid.cell_at(row, i).text))
            .collect();
        (schema, table, local_column, row_cells)
    };
    let token = model.mint_request_token();
    Update {
        render: true,
        effect: Some(Effect::LoadForeignKeys {
            request_token: token,
            session_id_hex,
            context_revision,
            schema,
            table,
            local_column,
            row_cells,
        }),
    }
}

fn show_structure(model: &mut Model) -> Update {
    let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
        return Update::unchanged();
    };
    let context_revision = model.workbench().context_revision;
    let Some((schema, table)) = relation_ddl_target(model) else {
        return Update::unchanged();
    };
    let token = model.mint_request_token();
    Update {
        render: true,
        effect: Some(Effect::LoadRelationStructure {
            request_token: token,
            session_id_hex,
            context_revision,
            schema,
            table,
        }),
    }
}

/// Reconstruct CREATE TABLE from open structure inspector and copy via OSC 52.
fn copy_structure_ddl(model: &mut Model) -> Update {
    use crate::model::structure_ddl::compose_create_table_ddl;

    let insp = &model.workbench().inspector;
    if !insp.open || insp.kind_label != "structure" {
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.error_label = Some("CopyDdl needs Structure panel open".into());
        }
        return Update::render();
    }
    let (Some(schema), Some(table)) = (
        insp.structure_schema.clone(),
        insp.structure_table.clone(),
    ) else {
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.error_label = Some("CopyDdl: no structure target".into());
        }
        return Update::render();
    };
    match compose_create_table_ddl(&schema, &table, &insp.text) {
        Ok(ddl) => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.error_label = Some(format!(
                    "copied CREATE TABLE {}.{} ({} bytes)",
                    schema,
                    table,
                    ddl.len()
                ));
            }
            Update {
                render: true,
                effect: Some(Effect::CopyToClipboard {
                    request_token: model.mint_request_token(),
                    text: ddl,
                }),
            }
        }
        Err(reason) => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.error_label = Some(format!("CopyDdl: {reason}"));
            }
            Update::render()
        }
    }
}

/// Resolve schema/table for DDL: grid base first, then structure inspector target.
/// Selected tree node is a group branch (`g:name`).
fn selected_connection_group(model: &Model) -> Option<String> {
    match model.profiles() {
        ProfileListState::Loaded {
            selected_id: Some(id),
            ..
        } if id.starts_with("g:") => {
            let name = id.strip_prefix("g:").unwrap_or("").to_owned();
            if name.is_empty() {
                None
            } else {
                Some(name)
            }
        }
        _ => None,
    }
}

/// Group names: 1–64 of [A-Za-z0-9._- ] (space allowed mid-name after trim).
fn is_safe_group_name(name: &str) -> bool {
    let t = name.trim();
    !t.is_empty()
        && t.len() <= 64
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ' '))
}

fn relation_ddl_target(model: &Model) -> Option<(String, String)> {
    if let Some(grid) = model.workbench().active_grid() {
        if let (Some(schema), Some(table)) =
            (grid.base_schema.clone(), grid.base_table.clone())
        {
            return Some((schema, table));
        }
    }
    let insp = &model.workbench().inspector;
    if insp.has_structure_target() {
        return Some((
            insp.structure_schema.clone()?,
            insp.structure_table.clone()?,
        ));
    }
    None
}

fn open_ddl_review(model: &mut Model, kind: &str, preview_fmt: &str) -> Update {
    let Some((schema, table)) = relation_ddl_target(model) else {
        return Update::unchanged();
    };
    model.set_confirm(Some(ConfirmDialog::DdlReview {
        kind: kind.into(),
        schema: schema.clone(),
        table: table.clone(),
        preview: preview_fmt
            .replace("{schema}", &schema)
            .replace("{table}", &table),
        confirm_buffer: String::new(),
    }));
    model.set_action(ActionId::Submit);
    Update::render()
}

fn open_redis_stage_confirm(model: &mut Model, add: bool) -> Update {
    if !model
        .workbench()
        .engine_kind
        .eq_ignore_ascii_case("Redis")
    {
        return Update::unchanged();
    }
    let Some((logical_db, key, kind_label)) = model.workbench().redis_stage_target.clone() else {
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.error_label = Some("open a redis hash/set/zset key first".into());
        }
        return Update::render();
    };
    let Some(op) = crate::model::redis_stage::op_for_kind(&kind_label, add) else {
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.error_label = Some(format!("stage add/remove unsupported for {kind_label}"));
        }
        return Update::render();
    };
    model.set_confirm(Some(ConfirmDialog::StageRedis {
        op: op.into(),
        logical_db,
        key,
        confirm_buffer: String::new(),
    }));
    model.set_action(ActionId::Submit);
    Update::render()
}

fn collect_mutation_specs(
    model: &Model,
) -> Option<(String, String, String, Vec<crate::effect::MutationChangeSpec>)> {
    use crate::effect::MutationChangeSpec;
    // Redis collection staging path (independent of relational grid editability).
    if model.workbench().engine_kind.eq_ignore_ascii_case("Redis") {
        let specs = model.workbench().redis_staged.clone();
        if specs.is_empty() {
            return None;
        }
        let (logical_db, key, _) = model.workbench().redis_stage_target.as_ref()?;
        return Some((logical_db.clone(), String::new(), key.clone(), specs));
    }
    let database = model.workbench().context.database.clone();
    let grid = model.workbench().active_grid()?;
    if !grid.editability.is_editable() || grid.drafts.is_empty() {
        return None;
    }
    let schema = grid.base_schema.clone()?;
    let table = grid.base_table.clone()?;
    let mut changes = Vec::new();
    for insert in &grid.drafts.inserts {
        changes.push(MutationChangeSpec::Insert {
            values: insert.values.clone(),
        });
    }
    let mut rows: Vec<u64> = grid.drafts.cell_edits.iter().map(|e| e.abs_row).collect();
    rows.sort_unstable();
    rows.dedup();
    for row in rows {
        let edits: Vec<_> = grid
            .drafts
            .cell_edits
            .iter()
            .filter(|e| e.abs_row == row)
            .collect();
        if edits.is_empty() {
            continue;
        }
        let locator = edits[0]
            .locator
            .iter()
            .map(|f| (f.column.clone(), f.original_text.clone()))
            .collect();
        let assignments = edits
            .iter()
            .map(|e| (e.column.clone(), e.staged_text.clone()))
            .collect();
        changes.push(MutationChangeSpec::Update {
            locator,
            assignments,
        });
    }
    for delete in &grid.drafts.deletes {
        changes.push(MutationChangeSpec::Delete {
            locator: delete
                .locator
                .iter()
                .map(|f| (f.column.clone(), f.original_text.clone()))
                .collect(),
        });
    }
    if changes.is_empty() {
        return None;
    }
    Some((database, schema, table, changes))
}

/// Review: register typed plan in process registry (handle for apply).
fn register_staged_review(model: &mut Model) -> Update {
    let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
        return Update::unchanged();
    };
    let context_revision = model.workbench().context_revision;
    let Some((database, schema, table, changes)) = collect_mutation_specs(model) else {
        return Update::unchanged();
    };
    let token = model.mint_request_token();
    // Clear prior token; new review supersedes.
    model.workbench_mut().pending_review_token_hex = None;
    model.workbench_mut().pending_review_expires_at_ms = None;
    Update {
        render: true,
        effect: Some(Effect::ReviewMutations {
            request_token: token,
            session_id_hex,
            context_revision,
            database,
            schema,
            table,
            changes,
        }),
    }
}

/// Apply by registry handle only — never rebuilds plan from drafts at apply time.
fn apply_staged_mutations(model: &mut Model) -> Update {
    let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
        return Update::unchanged();
    };
    let Some(review_token_hex) = model.workbench().pending_review_token_hex.clone() else {
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.error_label = Some("review required before apply".into());
        }
        return Update::render();
    };
    let context_revision = model.workbench().context_revision;
    let token = model.mint_request_token();
    if let Some(grid) = model.workbench_mut().active_grid_mut() {
        grid.operation = GridOperationState::Running;
        grid.error_label = None;
    }
    Update {
        render: true,
        effect: Some(Effect::ApplyMutations {
            request_token: token,
            session_id_hex,
            context_revision,
            review_token_hex,
        }),
    }
}

/// After connect-path restores (intent + filter library), load root catalog.
fn load_workbench_root_catalog(model: &mut Model) -> Update {
    let Some(session) = model.session() else {
        return Update::render();
    };
    let session_id_hex = session.session_id_hex.clone();
    let engine_label = model.workbench().engine_kind.clone();
    let token = model.mint_request_token();
    let context_revision = model.workbench().context_revision;
    model.workbench_mut().catalog = CatalogModel::Loading {
        request_token: token,
        context_revision,
    };
    Update {
        render: true,
        effect: Some(Effect::LoadCatalog {
            request_token: token,
            session_id_hex,
            context_revision,
            engine_label,
            level: crate::effect::CatalogLevelSpec::Root,
            parent_id: None,
        }),
    }
}

fn rebrowse_active_table(model: &mut Model) -> Update {
    let identity = model.workbench().active_grid().and_then(|g| {
        Some((g.base_schema.clone()?, g.base_table.clone()?))
    });
    let Some((schema, table)) = identity else {
        return Update::render();
    };
    let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
        return Update::render();
    };
    let token = model.mint_request_token();
    let context_revision = model.workbench().context_revision;
    if let Some(grid) = model.workbench_mut().active_grid_mut() {
        grid.operation = GridOperationState::Running;
        grid.error_label = None;
    }
    Update {
        render: true,
        effect: Some(browse_table_effect(
            token,
            session_id_hex,
            context_revision,
            schema,
            table,
            model.workbench().active_grid(),
        )),
    }
}

fn browse_table_effect(
    request_token: u64,
    session_id_hex: String,
    context_revision: u64,
    schema: String,
    table: String,
    grid: Option<&crate::model::grid::DataGridModel>,
) -> Effect {
    use crate::model::grid::ColumnSort;
    let (sort, filters, raw_where) = match grid {
        Some(g) => {
            let sort = g
                .sort
                .iter()
                .filter_map(|k| {
                    let dir = match k.direction {
                        ColumnSort::Asc => "asc",
                        ColumnSort::Desc => "desc",
                        ColumnSort::None => return None,
                    };
                    Some((k.column.clone(), dir.to_owned()))
                })
                .collect();
            let filters = g
                .filters
                .iter()
                .map(|f| (f.column.clone(), f.operator.clone(), f.value.clone()))
                .collect();
            (sort, filters, g.raw_where.clone())
        }
        None => (Vec::new(), Vec::new(), None),
    };
    Effect::BrowseTable {
        request_token,
        session_id_hex,
        context_revision,
        schema,
        table,
        sort,
        filters,
        raw_where,
    }
}

fn copy_grid(model: &mut Model, format: crate::model::copy_format::CopyFormat) -> Update {
    use crate::model::copy_format::{CopyScope, format_copy};
    let Some(grid) = model.workbench().active_grid() else {
        return Update::unchanged();
    };
    let text = match format_copy(grid, CopyScope::LoadedResult, format) {
        Ok(t) => t,
        Err(err) => {
            if let Some(g) = model.workbench_mut().active_grid_mut() {
                g.mark_failed(err.to_string());
            }
            return Update::render();
        }
    };
    let token = model.mint_request_token();
    Update {
        render: true,
        effect: Some(Effect::CopyToClipboard {
            request_token: token,
            text,
        }),
    }
}

fn copy_cursor_cell(model: &mut Model, hex: bool) -> Update {
    use crate::model::copy_format::{format_cursor_cell, format_cursor_cell_hex};
    let Some(grid) = model.workbench().active_grid() else {
        return Update::unchanged();
    };
    let text = match if hex {
        format_cursor_cell_hex(grid)
    } else {
        format_cursor_cell(grid)
    } {
        Ok(t) => t,
        Err(err) => {
            if let Some(g) = model.workbench_mut().active_grid_mut() {
                g.error_label = Some(err.to_string());
            }
            return Update::render();
        }
    };
    if let Some(g) = model.workbench_mut().active_grid_mut() {
        g.error_label = Some(if hex {
            format!("copied cell hex ({} bytes)", text.len() / 2)
        } else {
            format!("copied cell ({} bytes)", text.len())
        });
    }
    let token = model.mint_request_token();
    Update {
        render: true,
        effect: Some(Effect::CopyToClipboard {
            request_token: token,
            text,
        }),
    }
}

fn copy_cursor_row(model: &mut Model) -> Update {
    use crate::model::copy_format::{CopyFormat, CopyScope, format_copy};
    let Some(grid) = model.workbench().active_grid() else {
        return Update::unchanged();
    };
    let text = match format_copy(grid, CopyScope::Row, CopyFormat::Tsv) {
        Ok(t) => t,
        Err(err) => {
            if let Some(g) = model.workbench_mut().active_grid_mut() {
                g.error_label = Some(err.to_string());
            }
            return Update::render();
        }
    };
    if let Some(g) = model.workbench_mut().active_grid_mut() {
        g.error_label = Some(format!("copied row ({} bytes)", text.len()));
    }
    let token = model.mint_request_token();
    Update {
        render: true,
        effect: Some(Effect::CopyToClipboard {
            request_token: token,
            text,
        }),
    }
}

fn switch_workbench_database(model: &mut Model) -> Update {
    let Some(session) = model.session() else {
        return Update::unchanged();
    };
    let session_id_hex = session.session_id_hex.clone();
    let engine_label = model.workbench().engine_kind.clone();
    // Cycle known databases from catalog roots when available.
    let next_db = match &model.workbench().catalog {
        CatalogModel::Loaded { nodes, .. } => {
            let dbs: Vec<_> = nodes
                .iter()
                .filter(|n| n.depth == 0 && (n.kind_label == "database" || n.kind_label == "db"))
                .map(|n| n.label.clone())
                .collect();
            if dbs.is_empty() {
                None
            } else {
                let current = model.workbench().context.database.clone();
                let idx = dbs.iter().position(|d| d == &current).unwrap_or(0);
                Some(dbs[(idx + 1) % dbs.len()].clone())
            }
        }
        _ => None,
    };
    let Some(database) = next_db else {
        return Update::unchanged();
    };
    model.workbench_mut().context.database = database;
    let revision = model.workbench_mut().bump_context_revision();
    let token = model.mint_request_token();
    model.workbench_mut().catalog = CatalogModel::Loading {
        request_token: token,
        context_revision: revision,
    };
    Update {
        render: true,
        effect: Some(Effect::LoadCatalog {
            request_token: token,
            session_id_hex,
            context_revision: revision,
            engine_label,
            level: crate::effect::CatalogLevelSpec::Root,
            parent_id: None,
        }),
    }
}

fn cycle_action(
    screen: Screen,
    current: ActionId,
    reverse: bool,
    password_prompt: bool,
) -> ActionId {
    // Confirm and password prompts share Submit/Cancel actions.
    let modal = password_prompt; // caller passes password; confirm handled via selected actions
    let actions: &[ActionId] = if modal || matches!(current, ActionId::Submit | ActionId::Cancel) {
        &[ActionId::Submit, ActionId::Cancel, ActionId::Quit]
    } else {
        match screen {
            Screen::Editor => &[
                ActionId::Save,
                ActionId::Test,
                ActionId::Connect,
                ActionId::ImportUrl,
                ActionId::OpenExternalUrl,
                ActionId::Cancel,
                ActionId::Quit,
            ],
            Screen::Workbench => &[
                ActionId::NextDatabase,
                ActionId::NextTab,
                ActionId::QuickSwitch,
                ActionId::PinTab,
                ActionId::NewSql,
                ActionId::RunSql,
                ActionId::Explain,
                ActionId::FindReplace,
                ActionId::FormatSql,
                ActionId::Complete,
                ActionId::History,
                ActionId::RestoreHistory,
                ActionId::SavedQueries,
                ActionId::SaveQuery,
                ActionId::LoadQuery,
                ActionId::SaveFile,
                ActionId::SaveIntent,
                ActionId::CopyCsv,
                ActionId::CopyTsv,
                ActionId::CopyJson,
                ActionId::CopyCell,
                ActionId::CopyCellHex,
                ActionId::CopyRow,
                ActionId::CopyMarkdown,
                ActionId::CopySqlInsert,
                ActionId::CopySqlUpdate,
                ActionId::CycleSort,
                ActionId::AddFilter,
                ActionId::SaveFilter,
                ActionId::ApplyFilter,
                ActionId::ClearFilters,
                ActionId::ToggleColumn,
                ActionId::ResetColumns,
                ActionId::SaveColumns,
                ActionId::MoveColumnLeft,
                ActionId::MoveColumnRight,
                ActionId::NarrowColumn,
                ActionId::WidenColumn,
                ActionId::FitColumn,
                ActionId::FitAllColumns,
                ActionId::UndoStaged,
                ActionId::DiscardStaged,
                ActionId::ReviewMutations,
                ActionId::EditCell,
                ActionId::ToggleBool,
                ActionId::SetNull,
                ActionId::SetToday,
                ActionId::SetNow,
                ActionId::IncNumber,
                ActionId::DecNumber,
                ActionId::FormatJson,
                ActionId::CompactJson,
                ActionId::DeleteRow,
                ActionId::ApplyMutations,
                ActionId::FollowForeignKey,
                ActionId::ShowStructure,
                ActionId::CopyStructureDdl,
                ActionId::TruncateTable,
                ActionId::DropTable,
                ActionId::VacuumTable,
                ActionId::AnalyzeTable,
                ActionId::OptimizeTable,
                ActionId::DdlAddColumn,
                ActionId::DdlCreateIndex,
                ActionId::DdlDropColumn,
                ActionId::DdlDropIndex,
                ActionId::DdlAddConstraint,
                ActionId::DdlDropConstraint,
                ActionId::RenameTable,
                ActionId::ShowActivity,
                ActionId::ShowRoles,
                ActionId::CancelBackend,
                ActionId::TerminateBackend,
                ActionId::KillMutation,
                ActionId::ScanRedisKeys,
                ActionId::RedisInfo,
                ActionId::StageRedisAdd,
                ActionId::StageRedisRemove,
                ActionId::RedisCollectionMore,
                ActionId::ExportCsv,
                ActionId::ExportJson,
                ActionId::ExportTsv,
                ActionId::ExportStreamCsv,
                ActionId::ExportStreamJson,
                ActionId::ExportStreamTsv,
                ActionId::ImportCsv,
                ActionId::PgDump,
                ActionId::PgRestore,
                ActionId::CancelQuery,
                ActionId::Inspect,
                ActionId::CloseTab,
                ActionId::Disconnect,
                ActionId::SessionHealth,
                ActionId::Reconnect,
                ActionId::Quit,
            ],
            Screen::Connections | Screen::ConnectionPicker => &[
                ActionId::Open,
                ActionId::New,
                ActionId::ImportUrl,
                ActionId::OpenExternalUrl,
                ActionId::QuickSwitch,
                ActionId::Remove,
                ActionId::RenameGroup,
                ActionId::Reconnect,
                ActionId::Quit,
            ],
        }
    };
    let idx = actions.iter().position(|a| *a == current).unwrap_or(0);
    if reverse {
        actions[(idx + actions.len() - 1) % actions.len()]
    } else {
        actions[(idx + 1) % actions.len()]
    }
}

fn activate_catalog_node(model: &mut Model) -> Update {
    let Some(session) = model.session() else {
        return Update::unchanged();
    };
    let session_id_hex = session.session_id_hex.clone();
    let engine_label = model.workbench().engine_kind.clone();
    let context_revision = model.workbench().context_revision;
    let selected = match &model.workbench().catalog {
        CatalogModel::Loaded {
            selected_id: Some(id),
            nodes,
            ..
        } => nodes.iter().find(|n| n.id == *id).cloned(),
        CatalogModel::Loaded {
            selected_id: None,
            nodes,
            ..
        } if !nodes.is_empty() => {
            let first = nodes[0].clone();
            if let CatalogModel::Loaded { selected_id, .. } = &mut model.workbench_mut().catalog {
                *selected_id = Some(first.id.clone());
            }
            Some(first)
        }
        _ => None,
    };
    let Some(node) = selected else {
        return Update::unchanged();
    };
    if !node.branch {
        // Redis key leaves open a type-specific key view.
        let is_redis_key = matches!(
            node.kind_label.as_str(),
            "string" | "key" | "hash" | "list" | "set" | "zset" | "stream"
        ) || engine_label.eq_ignore_ascii_case("Redis")
            && !matches!(
                node.kind_label.as_str(),
                "database" | "db" | "namespace"
            );
        if is_redis_key {
            model.workbench_mut().open_preview_tab(node.label.clone());
            let token = model.mint_request_token();
            return Update {
                render: true,
                effect: Some(Effect::OpenRedisKey {
                    request_token: token,
                    session_id_hex,
                    context_revision,
                    key: node.label.clone(),
                    collection_skip: 0,
                }),
            };
        }
        // Data-bearing leaves open a browse tab; functions stay preview-only.
        let is_table = matches!(
            node.kind_label.as_str(),
            "table" | "view" | "matview" | "ftable"
        );
        model.workbench_mut().open_preview_tab(node.label.clone());
        if !is_table {
            return Update::render();
        }
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.operation = GridOperationState::Running;
        }
        let selected = model.workbench().selected_tab;
        if let Some(tab) = model.workbench_mut().tabs.get_mut(selected) {
            tab.running = true;
        }
        let token = model.mint_request_token();
        let context_revision = model.workbench().context_revision;
        // Path id: database/schema/table or schema/table.
        let parts: Vec<_> = node.id.split('/').collect();
        let (schema, table) = match parts.as_slice() {
            [.., schema, table] => ((*schema).to_owned(), (*table).to_owned()),
            [table] => (
                model
                    .workbench()
                    .context
                    .schema
                    .clone()
                    .unwrap_or_else(|| "public".into()),
                (*table).to_owned(),
            ),
            _ => ("public".into(), node.label.clone()),
        };
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some(schema.clone());
            grid.base_table = Some(table.clone());
            grid.ensure_column_layout();
        }
        // Prefer loading saved layout before first paint when profile known.
        if let Some(profile_id_hex) = model.workbench().profile_id_hex.clone() {
            let database = model.workbench().context.database.clone();
            return Update {
                render: true,
                effect: Some(Effect::LoadColumnLayout {
                    request_token: token,
                    profile_id_hex,
                    database,
                    schema: schema.clone(),
                    table: table.clone(),
                }),
            };
        }
        return Update {
            render: true,
            effect: Some(browse_table_effect(
                token,
                session_id_hex,
                context_revision,
                schema,
                table,
                model.workbench().active_grid(),
            )),
        };
    }
    let was_expanded = node.expanded;
    model.workbench_mut().catalog.toggle_expand(&node.id);
    if was_expanded {
        return Update::render();
    }
    let has_children = match &model.workbench().catalog {
        CatalogModel::Loaded { nodes, .. } => {
            let prefix = format!("{}/", node.id);
            nodes.iter().any(|n| n.id.starts_with(&prefix))
        }
        _ => false,
    };
    if has_children {
        return Update::render();
    }
    model
        .workbench_mut()
        .catalog
        .set_node_status(&node.id, CatalogNodeStatus::Loading);
    let level = catalog_level_for_expand(&engine_label, &node);
    let token = model.mint_request_token();
    if let CatalogModel::Loaded {
        request_token,
        context_revision: rev,
        ..
    } = &mut model.workbench_mut().catalog
    {
        *request_token = token;
        *rev = context_revision;
    }
    Update {
        render: true,
        effect: Some(Effect::LoadCatalog {
            request_token: token,
            session_id_hex,
            context_revision,
            engine_label,
            level,
            parent_id: Some(node.id),
        }),
    }
}

/// Move the grid viewport; emit FetchPage only when the target row is outside
/// the resident window (no I/O for pure resident scroll).
fn scroll_grid(model: &mut Model, direction: ScrollDirection) -> Update {
    let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
        return Update::unchanged();
    };
    let context_revision = model.workbench().context_revision;
    let fetch = {
        let Some(grid) = model.workbench_mut().active_grid_mut() else {
            return Update::unchanged();
        };
        match direction {
            ScrollDirection::Down => {
                grid.viewport_row = grid.viewport_row.saturating_add(1);
                grid.cursor_row = grid.cursor_row.saturating_add(1);
            }
            ScrollDirection::Up => {
                grid.viewport_row = grid.viewport_row.saturating_sub(1);
                grid.cursor_row = grid.cursor_row.saturating_sub(1);
            }
            ScrollDirection::Left => {
                grid.viewport_col = grid.viewport_col.saturating_sub(1);
                grid.cursor_col = grid.cursor_col.saturating_sub(1);
                return Update::render();
            }
            ScrollDirection::Right => {
                grid.viewport_col = grid.viewport_col.saturating_add(1);
                grid.cursor_col = grid.cursor_col.saturating_add(1);
                return Update::render();
            }
        }
        let target = grid.viewport_row.max(grid.cursor_row);
        if !grid.needs_fetch(target) || grid.result_token == 0 {
            None
        } else {
            Some((grid.next_fetch_start(), grid.result_token))
        }
    };
    let Some((start_row, result_token)) = fetch else {
        return Update::render();
    };
    let token = model.mint_request_token();
    Update {
        render: true,
        effect: Some(Effect::FetchPage {
            request_token: token,
            session_id_hex,
            context_revision,
            result_token,
            start_row,
        }),
    }
}

fn catalog_level_for_expand(
    engine_label: &str,
    node: &crate::model::catalog::CatalogNodeProjection,
) -> crate::effect::CatalogLevelSpec {
    use crate::effect::CatalogLevelSpec;
    match engine_label {
        "PostgreSQL" if node.kind_label == "database" => CatalogLevelSpec::Schemas {
            database: node.label.clone(),
        },
        "PostgreSQL" if node.kind_label == "schema" => {
            // id form: {database}/{schema}
            let database = node.id.split('/').next().unwrap_or("postgres").to_owned();
            CatalogLevelSpec::Relations {
                database,
                schema: node.label.clone(),
            }
        }
        "ClickHouse" if node.kind_label == "database" => CatalogLevelSpec::Objects {
            database: node.label.clone(),
        },
        _ => CatalogLevelSpec::Root,
    }
}

fn connection_draft_from_editor(
    editor: &crate::model::editor::ConnectionFormModel,
) -> crate::effect::ConnectionDraft {
    use crate::effect::{ConnectionDraft, PasswordSourceSpec, TlsModeSpec};
    use crate::model::editor::{PasswordSourceChoice, TlsModeChoice};
    ConnectionDraft {
        engine: editor.engine,
        name: editor.name.clone(),
        group: editor.group.clone(),
        environment: editor.environment.clone(),
        host: editor.host.clone(),
        port: editor.port.clone(),
        database: editor.database.clone(),
        username: editor.username.clone(),
        password: editor.password.clone(),
        password_source: match editor.password_source {
            PasswordSourceChoice::PromptOnConnect => PasswordSourceSpec::PromptOnConnect,
            PasswordSourceChoice::HostEnvironment => PasswordSourceSpec::HostEnvironment {
                var: editor.password.clone(),
            },
            PasswordSourceChoice::OnePassword => {
                match tablerock_core::OnePasswordReference::from_compact_wire(
                    editor.password.trim(),
                ) {
                    Ok(reference) => PasswordSourceSpec::OnePassword {
                        account_id: reference.account_id().as_str().to_owned(),
                        vault_id: reference.vault_id().as_str().to_owned(),
                        item_id: reference.item_id().as_str().to_owned(),
                        section_id: reference
                            .section_id()
                            .map(|s| s.as_str().to_owned()),
                        field_id: reference.field_id().as_str().to_owned(),
                        breadcrumb: reference.breadcrumb().to_owned(),
                    },
                    // Validation should have caught this; keep a safe empty stub.
                    Err(_) => PasswordSourceSpec::OnePassword {
                        account_id: String::new(),
                        vault_id: String::new(),
                        item_id: String::new(),
                        section_id: None,
                        field_id: String::new(),
                        breadcrumb: String::new(),
                    },
                }
            }
            PasswordSourceChoice::DangerousPlaintext => PasswordSourceSpec::DangerousPlaintext,
        },
        tls_mode: match editor.tls_mode {
            TlsModeChoice::Off => TlsModeSpec::Off,
            TlsModeChoice::VerifyCa => TlsModeSpec::VerifyCa,
            TlsModeChoice::VerifyFull => TlsModeSpec::VerifyFull,
        },
        plaintext_acknowledged: editor.plaintext_acknowledged,
        ssh_host: editor.ssh_host.clone(),
        ssh_port: editor.ssh_port.clone(),
        ssh_username: editor.ssh_username.clone(),
        ssh_password: editor.ssh_password.clone(),
        ssh_private_key: editor.ssh_private_key.clone(),
        ssh_known_hosts_path: editor.ssh_known_hosts_path.clone(),
        ssh_use_agent: editor.ssh_use_agent,
        startup_actions: editor
            .startup_action_set()
            .unwrap_or_else(|_| tablerock_core::StartupActionSet::empty()),
        reconnect_preference: "Manual".into(),
    }
}

fn apply_editor_text(model: &mut Model, text: &str) {
    use crate::model::editor::EditorField;
    let editor = model.editor_mut();
    match editor.focused {
        EditorField::Engine => editor.cycle_engine(),
        EditorField::Name => editor.name.push_str(text),
        EditorField::Group => editor.group.push_str(text),
        EditorField::Environment => editor.environment.push_str(text),
        EditorField::Host => editor.host.push_str(text),
        EditorField::Port => editor.port.push_str(text),
        EditorField::Database => editor.database.push_str(text),
        EditorField::Username => editor.username.push_str(text),
        EditorField::Password => editor.password.push_str(text),
        EditorField::PasswordSource => {
            editor.password_source = match editor.password_source {
                crate::model::editor::PasswordSourceChoice::PromptOnConnect => {
                    crate::model::editor::PasswordSourceChoice::HostEnvironment
                }
                crate::model::editor::PasswordSourceChoice::HostEnvironment => {
                    crate::model::editor::PasswordSourceChoice::OnePassword
                }
                crate::model::editor::PasswordSourceChoice::OnePassword => {
                    crate::model::editor::PasswordSourceChoice::DangerousPlaintext
                }
                crate::model::editor::PasswordSourceChoice::DangerousPlaintext => {
                    crate::model::editor::PasswordSourceChoice::PromptOnConnect
                }
            };
        }
        EditorField::TlsMode => {
            editor.tls_mode = match editor.tls_mode {
                crate::model::editor::TlsModeChoice::Off => {
                    crate::model::editor::TlsModeChoice::VerifyCa
                }
                crate::model::editor::TlsModeChoice::VerifyCa => {
                    crate::model::editor::TlsModeChoice::VerifyFull
                }
                crate::model::editor::TlsModeChoice::VerifyFull => {
                    crate::model::editor::TlsModeChoice::Off
                }
            };
        }
        EditorField::SshHost => editor.ssh_host.push_str(text),
        EditorField::SshPort => editor.ssh_port.push_str(text),
        EditorField::SshUsername => editor.ssh_username.push_str(text),
        EditorField::SshPassword => editor.ssh_password.push_str(text),
        EditorField::SshPrivateKey => editor.ssh_private_key.push_str(text),
        EditorField::SshKnownHostsPath => editor.ssh_known_hosts_path.push_str(text),
        EditorField::SshUseAgent => {
            editor.ssh_use_agent = !editor.ssh_use_agent;
        }
        EditorField::StartupSql => editor.startup_sql.push_str(text),
    }
}

fn maybe_bootstrap_profiles(model: &mut Model) -> Update {
    if model.bootstrapped() || model.screen() != Screen::Connections {
        return Update::unchanged();
    }
    model.set_bootstrapped(true);
    let token = model.mint_request_token();
    model.set_profiles(ProfileListState::Loading {
        request_token: token,
    });
    Update::with_effect(Effect::LoadProfileList {
        request_token: token,
        filter: ProfileListFilterSpec::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PasteText;
    use crate::model::profiles::{FailureProjection, ProfileRowProjection};

    #[test]
    fn bootstrap_emits_load_and_rejects_stale_tokens() {
        let mut model = Model::default();
        let first = update(
            &mut model,
            Message::Resize {
                width: 80,
                height: 24,
            },
        );
        assert!(matches!(
            first.effects().next(),
            Some(Effect::LoadProfileList {
                request_token: 1,
                ..
            })
        ));
        assert!(matches!(
            model.profiles(),
            ProfileListState::Loading { request_token: 1 }
        ));

        // Stale completion ignored.
        let stale = update(
            &mut model,
            Message::Profiles(ProfilesMsg::ListLoaded {
                request_token: 99,
                items: vec![],
            }),
        );
        assert!(!stale.needs_render());
        assert!(matches!(
            model.profiles(),
            ProfileListState::Loading { request_token: 1 }
        ));

        let ok = update(
            &mut model,
            Message::Profiles(ProfilesMsg::ListLoaded {
                request_token: 1,
                items: vec![ProfileRowProjection {
                    id_hex: "1".into(),
                    name: "a".into(),
                    engine_label: "PostgreSQL".into(),
                    group: None,
                    favorite: false,
                    target_summary: "localhost:5432".into(),
                    environment: None,
                    production_warning: false,
                    safety_label: "Read only".into(),
                    plaintext_secret_warning: false,
                    live_state: crate::model::profiles::LiveConnectionState::Disconnected,
                }],
            }),
        );
        assert!(ok.needs_render());
        assert_eq!(model.profiles().status_line(), "Profiles: 1");
    }

    #[test]
    fn failure_label_is_redacted_status_only() {
        let mut model = Model::default();
        let _ = update(
            &mut model,
            Message::Resize {
                width: 80,
                height: 24,
            },
        );
        let _ = update(
            &mut model,
            Message::Profiles(ProfilesMsg::ListFailed {
                request_token: 1,
                reason: FailureProjection::Label("unavailable".into()),
            }),
        );
        assert_eq!(
            model.profiles().status_line(),
            "Profiles: error (unavailable)"
        );
    }

    #[test]
    fn test_action_emits_effect_and_records_outcome() {
        let mut model = Model::default();
        model.set_screen(Screen::Editor);
        model.set_action(ActionId::Test);
        model.editor_mut().name = "local".into();
        model.editor_mut().host = "127.0.0.1".into();
        model.editor_mut().port = "5432".into();
        // Focus Actions so Activate would work; call action path via ActionId match.
        for _ in 0..4 {
            let _ = update(&mut model, Message::FocusNext);
        }
        // selected action may have moved; set Test again after focus moves to Actions
        model.set_action(ActionId::Test);
        let result = update(&mut model, Message::Activate);
        assert!(matches!(
            result.effects().next(),
            Some(Effect::TestConnection {
                request_token: 1,
                ..
            })
        ));
        assert_eq!(model.editor().test_status.as_deref(), Some("testing…"));

        let ok = update(
            &mut model,
            Message::Engine(EngineMsg::TestOk {
                request_token: 1,
                identity: "PostgreSQL 17".into(),
                elapsed_millis: 12,
                startup_summary: None,
            }),
        );
        assert!(ok.needs_render());
        assert_eq!(
            model.editor().test_status.as_deref(),
            Some("ok: PostgreSQL 17 (12 ms)")
        );

        let with_startup = update(
            &mut model,
            Message::Engine(EngineMsg::TestOk {
                request_token: 1,
                identity: "PostgreSQL 17".into(),
                elapsed_millis: 9,
                startup_summary: Some("startup 1ok/0skip/0fail/0timeout".into()),
            }),
        );
        assert!(with_startup.needs_render());
        assert_eq!(
            model.editor().test_status.as_deref(),
            Some("ok: PostgreSQL 17 (9 ms); startup 1ok/0skip/0fail/0timeout")
        );

        let fail = update(
            &mut model,
            Message::Engine(EngineMsg::TestFailed {
                request_token: 1,
                reason: FailureProjection::Label("connect".into()),
            }),
        );
        assert!(fail.needs_render());
        assert_eq!(
            model.editor().test_status.as_deref(),
            Some("failed: connect")
        );
    }

    #[test]
    fn remove_requires_confirm_then_emits_delete() {
        let mut model = Model::default();
        model.set_screen(Screen::Connections);
        model.set_profiles(ProfileListState::Loaded {
            request_token: 1,
            rows: vec![crate::model::profiles::ProfileRowProjection {
                id_hex: "aa".into(),
                name: "local".into(),
                engine_label: "PostgreSQL".into(),
                group: None,
                favorite: false,
                target_summary: "127.0.0.1:5432".into(),
                environment: None,
                production_warning: false,
                safety_label: "Confirm writes".into(),
                plaintext_secret_warning: false,
                live_state: crate::model::profiles::LiveConnectionState::Disconnected,
            }],
            selected_id: Some("aa".into()),
            search: String::new(),
            collapsed: Vec::new(),
        });
        model.set_action(ActionId::Remove);
        for _ in 0..4 {
            let _ = update(&mut model, Message::FocusNext);
        }
        model.set_action(ActionId::Remove);
        let ask = update(&mut model, Message::Activate);
        assert!(ask.effects().next().is_none());
        assert!(model.confirm().is_some());
        model.set_action(ActionId::Submit);
        let del = update(&mut model, Message::Activate);
        assert!(matches!(
            del.effects().next(),
            Some(Effect::DeleteProfile {
                profile_id_hex,
                ..
            }) if profile_id_hex == "aa"
        ));
    }

    #[test]
    fn redis_run_pipeline_emits_effect_and_denies_blocking() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "0000000000000001000000000000000f".into(),
            identity: "redis".into(),
            temporary: true,
            engine_label: "Redis".into(),
            status: Some("connected".into()),
        }));
        model.workbench_mut().engine_kind = "Redis".into();
        model.workbench_mut().open_sql_tab();
        assert_eq!(model.workbench().active_tab().unwrap().title, "Redis");
        if let Some(ed) = model.workbench_mut().active_editor_mut() {
            ed.set_text("SET k v\nGET k\n");
            ed.set_cursor(ed.text().len());
        }
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::RunSql);
        let out = update(&mut model, Message::Activate);
        match out.effects().next() {
            Some(Effect::ExecuteRedisPipeline { commands, .. }) => {
                assert_eq!(commands.len(), 2);
                assert_eq!(commands[0].0, "SET");
                assert_eq!(commands[1].0, "GET");
            }
            other => panic!("expected ExecuteRedisPipeline, got {other:?}"),
        }
        // Lone BLPOP → disposable-connection effect (not shared-session deny).
        if let Some(ed) = model.workbench_mut().active_editor_mut() {
            ed.set_text("BLPOP q 0\n");
            ed.set_cursor(ed.text().len());
        }
        model.set_action(ActionId::RunSql);
        let isolated = update(&mut model, Message::Activate);
        match isolated.effects().next() {
            Some(Effect::RedisBlockingPop { key, .. }) => assert_eq!(&*key, "q"),
            other => panic!("expected RedisBlockingPop, got {other:?}"),
        }
        // Mixed pipeline with blocking still denied.
        if let Some(ed) = model.workbench_mut().active_editor_mut() {
            ed.set_text("GET k\nBLPOP q 0\n");
            ed.set_cursor(ed.text().len());
        }
        model.set_action(ActionId::RunSql);
        let blocked = update(&mut model, Message::Activate);
        assert!(blocked.effects().next().is_none());
        assert!(model
            .workbench()
            .active_grid()
            .and_then(|g| g.error_label.as_deref())
            .is_some_and(|l| l.contains("blocking")));
    }

    #[test]
    fn redis_pipeline_done_fills_sections_and_inspector() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.workbench_mut().context_revision = 1;
        model.workbench_mut().open_sql_tab();
        let out = update(
            &mut model,
            Message::Engine(EngineMsg::RedisPipelineDone {
                request_token: 1,
                context_revision: 1,
                lines: vec![
                    "1. ok SET (2 args) → OK".into(),
                    "2. ERR BLPOP (1 arg) → blocking".into(),
                ],
                ok_count: 1,
                fail_count: 1,
            }),
        );
        assert!(out.needs_render());
        assert_eq!(model.workbench().result_sections.sections.len(), 2);
        assert!(model.workbench().inspector.open);
        assert!(model.workbench().inspector.title.contains("1ok/1err"));
    }

    #[test]
    fn redis_collection_more_uses_stored_skip() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "0000000000000001000000000000000e".into(),
            identity: "redis".into(),
            temporary: true,
            engine_label: "Redis".into(),
            status: Some("connected".into()),
        }));
        model.workbench_mut().engine_kind = "Redis".into();
        model.workbench_mut().redis_stage_target =
            Some(("0".into(), "bigset".into(), "set".into()));
        model.workbench_mut().redis_collection_skip = Some(32);
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::RedisCollectionMore);
        let out = update(&mut model, Message::Activate);
        match out.effects().next() {
            Some(Effect::OpenRedisKey {
                key,
                collection_skip,
                ..
            }) => {
                assert_eq!(&*key, "bigset");
                assert_eq!(*collection_skip, 32);
            }
            other => panic!("expected OpenRedisKey with skip, got {other:?}"),
        }
        // No skip → fail closed label.
        model.workbench_mut().redis_collection_skip = None;
        model.set_action(ActionId::RedisCollectionMore);
        let _ = update(&mut model, Message::Activate);
        assert!(model
            .workbench()
            .active_grid()
            .and_then(|g| g.error_label.as_deref())
            .is_some_and(|l| l.contains("no more")));
    }

    #[test]
    fn stage_redis_hash_add_then_review_emits_collection_specs() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "0000000000000001000000000000000d".into(),
            identity: "redis".into(),
            temporary: true,
            engine_label: "Redis".into(),
            status: Some("connected".into()),
        }));
        model.workbench_mut().engine_kind = "Redis".into();
        model.workbench_mut().context.database = "0".into();
        let rev = model.workbench().context_revision;
        // Simulate OpenRedisKey load for a hash.
        let _ = update(
            &mut model,
            Message::Engine(EngineMsg::RedisKeyViewLoaded {
                request_token: 1,
                context_revision: rev,
                key: "myhash".into(),
                kind_label: "hash".into(),
                lines: vec!["type: Hash".into(), "HSCAN page skip=0 take=0 (end)".into()],
                next_collection_skip: None,
            }),
        );
        assert_eq!(
            model.workbench().redis_stage_target.as_ref().map(|t| (&t.0, &t.1, &t.2)),
            Some((&"0".to_owned(), &"myhash".to_owned(), &"hash".to_owned()))
        );
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::StageRedisAdd);
        let ask = update(&mut model, Message::Activate);
        assert!(ask.effects().next().is_none());
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::StageRedis { op, key, .. })
                if op == "hset" && key == "myhash"
        ));
        if let Some(ConfirmDialog::StageRedis {
            confirm_buffer, ..
        }) = model.confirm_mut()
        {
            *confirm_buffer = "field1=hello".into();
        }
        model.set_action(ActionId::Submit);
        let staged = update(&mut model, Message::Activate);
        assert!(staged.effects().next().is_none());
        assert_eq!(model.workbench().redis_staged.len(), 1);
        assert!(matches!(
            &model.workbench().redis_staged[0],
            crate::effect::MutationChangeSpec::RedisHashSet { field, value }
                if field == "field1" && value == "hello"
        ));
        model.set_action(ActionId::ReviewMutations);
        let review = update(&mut model, Message::Activate);
        match review.effects().next() {
            Some(Effect::ReviewMutations {
                database,
                table,
                changes,
                ..
            }) => {
                assert_eq!(database, "0");
                assert_eq!(table, "myhash");
                assert_eq!(changes.len(), 1);
                assert!(matches!(
                    &changes[0],
                    crate::effect::MutationChangeSpec::RedisHashSet { .. }
                ));
            }
            other => panic!("expected ReviewMutations with redis specs, got {other:?}"),
        }
        // Discard clears redis staged.
        model.set_action(ActionId::DiscardStaged);
        let _ = update(&mut model, Message::Activate);
        assert!(model.workbench().redis_staged.is_empty());
    }

    #[test]
    fn scan_redis_keys_uses_catalog_filter_as_match_pattern() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "0000000000000001000000000000000c".into(),
            identity: "redis".into(),
            temporary: true,
            engine_label: "Redis".into(),
            status: Some("connected".into()),
        }));
        model.workbench_mut().engine_kind = "Redis".into();
        model.workbench_mut().catalog = CatalogModel::Loaded {
            request_token: 1,
            context_revision: 1,
            nodes: Vec::new(),
            selected_id: None,
            filter: "user:*".into(),
            truncated: false,
        };
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::ScanRedisKeys);
        let out = update(&mut model, Message::Activate);
        match out.effects().next() {
            Some(Effect::ScanRedisKeys {
                pattern,
                count,
                ..
            }) => {
                assert_eq!(&*pattern, "user:*");
                assert_eq!(*count, 100);
            }
            other => panic!("expected ScanRedisKeys with MATCH pattern, got {other:?}"),
        }
    }

    #[test]
    fn save_and_apply_named_filter_preset_round_trip() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000003".into(),
            identity: "pg".into(),
            temporary: false,
            engine_label: "PostgreSQL".into(),
            status: Some("connected".into()),
        }));
        model.workbench_mut().profile_id_hex =
            Some("0000000000000001000000000000000a".into());
        model.workbench_mut().open_preview_tab("users");
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("public".into());
            grid.base_table = Some("users".into());
            grid.columns = vec!["status".into()];
            grid.add_filter_chip("status", "eq", Some("active".into()));
        }
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::SaveFilter);
        let ask = update(&mut model, Message::Activate);
        assert!(ask.effects().next().is_none());
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::SaveFilter { .. })
        ));
        if let Some(ConfirmDialog::SaveFilter {
            confirm_buffer, ..
        }) = model.confirm_mut()
        {
            *confirm_buffer = "active_only".into();
        }
        model.set_action(ActionId::Submit);
        let save = update(&mut model, Message::Activate);
        match save.effects().next() {
            Some(Effect::SaveSavedFilterLibrary {
                profile_id_hex,
                library_json,
                ..
            }) => {
                assert_eq!(profile_id_hex, "0000000000000001000000000000000a");
                assert!(library_json.contains("active_only"));
                assert!(library_json.contains("status"));
            }
            other => panic!("expected SaveSavedFilterLibrary, got {other:?}"),
        }
        // Clear live filters then apply named preset.
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.clear_server_controls();
            assert!(grid.filters.is_empty());
        }
        model.set_action(ActionId::ApplyFilter);
        let ask_apply = update(&mut model, Message::Activate);
        assert!(ask_apply.effects().next().is_none());
        if let Some(ConfirmDialog::ApplyFilter {
            known_names,
            confirm_buffer,
            ..
        }) = model.confirm_mut()
        {
            assert!(known_names.iter().any(|n| n == "active_only"));
            *confirm_buffer = "active_only".into();
        } else {
            panic!("expected ApplyFilter confirm");
        }
        model.set_action(ActionId::Submit);
        let apply = update(&mut model, Message::Activate);
        assert!(matches!(
            apply.effects().next(),
            Some(Effect::BrowseTable { .. })
        ));
        let grid = model.workbench().active_grid().unwrap();
        assert_eq!(grid.filters.len(), 1);
        assert_eq!(grid.filters[0].column, "status");
        assert_eq!(grid.filters[0].value.as_deref(), Some("active"));
        // Unique fuzzy resolve: partial buffer → single match.
        model.workbench_mut().filter_library.upsert(
            crate::model::saved_filter::SavedFilterPreset {
                name: "archived".into(),
                schema: "public".into(),
                table: "users".into(),
                filters: vec![crate::model::grid::GridFilterChip {
                    column: "status".into(),
                    operator: "eq".into(),
                    value: Some("archived".into()),
                }],
                raw_where: None,
            },
        );
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.clear_server_controls();
        }
        model.set_action(ActionId::ApplyFilter);
        let _ = update(&mut model, Message::Activate);
        if let Some(ConfirmDialog::ApplyFilter {
            confirm_buffer, ..
        }) = model.confirm_mut()
        {
            *confirm_buffer = "arch".into();
        }
        model.set_action(ActionId::Submit);
        let fuzzy = update(&mut model, Message::Activate);
        assert!(matches!(
            fuzzy.effects().next(),
            Some(Effect::BrowseTable { .. })
        ));
        assert_eq!(
            model
                .workbench()
                .active_grid()
                .unwrap()
                .filters
                .first()
                .and_then(|f| f.value.as_deref()),
            Some("archived")
        );
    }

    #[test]
    fn apply_filter_ambiguous_fuzzy_keeps_dialog() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.workbench_mut().open_preview_tab("users");
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("public".into());
            grid.base_table = Some("users".into());
        }
        for name in ["active_only", "active_all"] {
            model.workbench_mut().filter_library.upsert(
                crate::model::saved_filter::SavedFilterPreset {
                    name: name.into(),
                    schema: "public".into(),
                    table: "users".into(),
                    filters: Vec::new(),
                    raw_where: None,
                },
            );
        }
        model.set_action(ActionId::ApplyFilter);
        let _ = model.request_focus(FocusRegion::Actions);
        let _ = update(&mut model, Message::Activate);
        if let Some(ConfirmDialog::ApplyFilter {
            confirm_buffer, ..
        }) = model.confirm_mut()
        {
            *confirm_buffer = "act".into();
        }
        model.set_action(ActionId::Submit);
        let out = update(&mut model, Message::Activate);
        assert!(out.effects().next().is_none());
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::ApplyFilter { .. })
        ));
    }

    #[test]
    fn loaded_filter_library_restores_presets_then_catalog() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000004".into(),
            identity: "pg".into(),
            temporary: false,
            engine_label: "PostgreSQL".into(),
            status: Some("connected".into()),
        }));
        model.workbench_mut().profile_id_hex =
            Some("0000000000000001000000000000000b".into());
        let json = r#"[{"name":"default","schema":"public","table":"users","raw_where":null,"filters":[{"column":"id","operator":"eq","value":"1"}]}]"#;
        let out = update(
            &mut model,
            Message::Engine(EngineMsg::SavedFilterLibraryLoaded {
                request_token: 1,
                library_json: Some(json.into()),
            }),
        );
        assert!(model.workbench().filter_library.get("default", "public", "users").is_some());
        assert!(matches!(out.effects().next(), Some(Effect::LoadCatalog { .. })));
    }

    #[test]
    fn kill_mutation_requires_retype_and_emits_effect() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000002".into(),
            identity: "ch".into(),
            temporary: true,
            engine_label: "ClickHouse".into(),
            status: Some("connected".into()),
        }));
        model.workbench_mut().open_preview_tab("kill_mut");
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("default".into());
            grid.base_table = Some("kill_mut".into());
        }
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::KillMutation);
        let ask = update(&mut model, Message::Activate);
        assert!(ask.effects().next().is_none());
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::KillMutation {
                database,
                table,
                ..
            }) if database == "default" && table == "kill_mut"
        ));
        // Wrong charset stays open, no effect.
        if let Some(ConfirmDialog::KillMutation {
            confirm_buffer, ..
        }) = model.confirm_mut()
        {
            *confirm_buffer = "bad;drop".into();
        }
        model.set_action(ActionId::Submit);
        let reject = update(&mut model, Message::Activate);
        assert!(reject.effects().next().is_none());
        assert!(model.confirm().is_some());
        // Valid mutation id → KillClickHouseMutation effect.
        if let Some(ConfirmDialog::KillMutation {
            confirm_buffer, ..
        }) = model.confirm_mut()
        {
            *confirm_buffer = "mutation_2.txt".into();
        }
        model.set_action(ActionId::Submit);
        let kill = update(&mut model, Message::Activate);
        match kill.effects().next() {
            Some(Effect::KillClickHouseMutation {
                database,
                table,
                mutation_id,
                ..
            }) => {
                assert_eq!(database, "default");
                assert_eq!(table, "kill_mut");
                assert_eq!(mutation_id, "mutation_2.txt");
            }
            other => panic!("expected KillClickHouseMutation, got {other:?}"),
        }
        assert!(model.confirm().is_none());
    }

    #[test]
    fn password_prompt_debug_redacts_buffer_and_submit_clears() {
        let mut model = Model::default();
        let _ = update(
            &mut model,
            Message::Engine(EngineMsg::PasswordPromptRequired {
                request_token: 7,
                profile_id_hex: "abc".into(),
            }),
        );
        assert!(model.password_prompt().is_some());
        if let Some(prompt) = model.password_prompt_mut() {
            prompt.buffer.push_str("super-secret");
        }
        let debug = format!("{:?}", model.password_prompt());
        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("buffer_bytes"));
        model.set_action(ActionId::Submit);
        for _ in 0..4 {
            let _ = update(&mut model, Message::FocusNext);
        }
        model.set_action(ActionId::Submit);
        let result = update(&mut model, Message::Activate);
        assert!(matches!(
            result.effects().next(),
            Some(Effect::ResumeConnectProfile {
                password,
                ..
            }) if password == "super-secret"
        ));
        assert!(model.password_prompt().is_none());
    }

    #[test]
    fn temporary_connect_effect_sets_temporary_flag() {
        let mut model = Model::default();
        model.set_screen(Screen::Editor);
        model.set_action(ActionId::Connect);
        model.editor_mut().name = "tmp".into();
        model.editor_mut().host = "127.0.0.1".into();
        model.editor_mut().port = "5432".into();
        for _ in 0..4 {
            let _ = update(&mut model, Message::FocusNext);
        }
        model.set_action(ActionId::Connect);
        let result = update(&mut model, Message::Activate);
        assert!(matches!(
            result.effects().next(),
            Some(Effect::ConnectSession {
                temporary: true,
                ..
            })
        ));
    }

    #[test]
    fn connect_opens_workbench_and_disconnect_returns() {
        let mut model = Model::default();
        model.set_screen(Screen::Editor);
        let result = update(
            &mut model,
            Message::Engine(EngineMsg::ConnectOk {
                request_token: 1,
                session_id_hex: "00000000000000010000000000000001".into(),
                identity: "PostgreSQL 17".into(),
                temporary: true,
                engine_label: "PostgreSQL".into(),
                profile_id_hex: None,
                startup_summary: None,
                startup_pending: Vec::new(),
                reconnect_preference: Some("Manual".into()),
            }),
        );
        assert_eq!(model.screen(), Screen::Workbench);
        assert!(model.session().is_some());
        assert!(model.workbench().context.line().contains("PostgreSQL"));
        assert_eq!(model.selected_action(), ActionId::Disconnect);
        assert!(matches!(
            result.effects().next(),
            Some(Effect::LoadCatalog {
                engine_label,
                level: crate::effect::CatalogLevelSpec::Root,
                ..
            }) if engine_label == "PostgreSQL"
        ));
        assert!(matches!(
            model.workbench().catalog,
            CatalogModel::Loading { .. }
        ));

        let _ = update(
            &mut model,
            Message::Engine(EngineMsg::DisconnectOk {
                request_token: 2,
                session_id_hex: "00000000000000010000000000000001".into(),
            }),
        );
        assert_eq!(model.screen(), Screen::Connections);
        assert!(model.session().is_none());
    }

    #[test]
    fn disconnect_mid_stream_ignores_late_grid_pages() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000009".into(),
            identity: "pg".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: Some("connected".into()),
        }));
        model.workbench_mut().context_revision = 3;
        model.workbench_mut().open_preview_tab("live");
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.operation = GridOperationState::Streaming;
            grid.columns = vec!["id".into()];
            grid.cells = vec![crate::model::grid::ProjectedCell {
                text: "1".into(),
                distinction: crate::model::grid::CellDistinction::Number,
                byte_len: 1,
                original_byte_len: None,
            }];
            grid.row_count = 1;
            grid.rows_loaded = 1;
        }
        model.workbench_mut().mark_active_running(true);
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::Disconnect);
        let disc = update(&mut model, Message::Activate);
        assert!(matches!(
            disc.effects().next(),
            Some(Effect::DisconnectSession { .. })
        ));
        assert_eq!(
            model.workbench().active_grid().unwrap().operation,
            GridOperationState::Disconnected
        );
        // Late page must not overwrite disconnected state or cells.
        let late = update(
            &mut model,
            Message::Engine(EngineMsg::GridPage {
                request_token: 9,
                context_revision: 3,
                start_row: 0,
                columns: vec!["id".into()],
                cells: vec![crate::model::grid::ProjectedCell {
                    text: "999".into(),
                    distinction: crate::model::grid::CellDistinction::Number,
                    byte_len: 3,
                    original_byte_len: None,
                }],
                row_count: 1,
                totals_exact: Some(1),
                totals_estimated: None,
                bytes: 8,
                truncated: false,
                complete: true,
                identity_columns: None,
                server_query_id: None,
                server_progress: None,
            }),
        );
        assert!(!late.needs_render());
        let grid = model.workbench().active_grid().unwrap();
        assert_eq!(grid.operation, GridOperationState::Disconnected);
        assert_eq!(grid.cells[0].text, "1");
        // Late complete also ignored.
        let done = update(
            &mut model,
            Message::Engine(EngineMsg::GridStreamComplete {
                request_token: 9,
                context_revision: 3,
                rows_loaded: 50,
                truncated: false,
                notice_summary: None,
            }),
        );
        assert!(!done.needs_render());
        assert_eq!(
            model.workbench().active_grid().unwrap().operation,
            GridOperationState::Disconnected
        );
        assert_eq!(model.workbench().active_grid().unwrap().rows_loaded, 1);
    }

    #[test]
    fn catalog_loaded_merges_roots_and_rejects_stale_revision() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.workbench_mut().context_revision = 2;
        model.workbench_mut().catalog = CatalogModel::Loading {
            request_token: 5,
            context_revision: 2,
        };
        let stale = update(
            &mut model,
            Message::Engine(EngineMsg::CatalogLoaded {
                request_token: 5,
                context_revision: 1,
                parent_id: None,
                nodes: vec![],
                truncated: false,
            }),
        );
        assert!(!stale.needs_render());
        assert!(matches!(
            model.workbench().catalog,
            CatalogModel::Loading { .. }
        ));

        let ok = update(
            &mut model,
            Message::Engine(EngineMsg::CatalogLoaded {
                request_token: 5,
                context_revision: 2,
                parent_id: None,
                nodes: vec![crate::model::catalog::CatalogNodeProjection {
                    id: "postgres".into(),
                    label: "postgres".into(),
                    kind_label: "database".into(),
                    depth: 0,
                    branch: true,
                    expanded: false,
                    status: CatalogNodeStatus::Ready,
                }],
                truncated: false,
            }),
        );
        assert!(ok.needs_render());
        match &model.workbench().catalog {
            CatalogModel::Loaded { nodes, .. } => {
                assert_eq!(nodes.len(), 1);
                assert_eq!(nodes[0].label, "postgres");
            }
            other => panic!("expected loaded, got {other:?}"),
        }
    }

    #[test]
    fn resident_scroll_does_not_request_fetch() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000001".into(),
            identity: "pg".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: Some("connected".into()),
        }));
        model.workbench_mut().open_preview_tab("t");
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.replace_page(
                0,
                vec!["id".into()],
                (0..10)
                    .map(|i| crate::model::grid::ProjectedCell {
                        text: i.to_string(),
                        distinction: crate::model::grid::CellDistinction::Number,
                        byte_len: 1,
                        original_byte_len: None,
                    })
                    .collect(),
                10,
                GridRowTotal::Exact(10),
                10,
                false,
            );
            grid.operation = GridOperationState::Completed;
            grid.result_token = 7;
            grid.viewport_row = 0;
        }
        // Scroll inside resident window → no FetchPage effect.
        let scrolled = update(
            &mut model,
            Message::PointerScrolled {
                target: Some(ShellTarget::Focus(FocusRegion::Content)),
                direction: crate::ScrollDirection::Down,
            },
        );
        assert!(scrolled.effects().next().is_none());
        assert!(!model.workbench().active_grid().unwrap().needs_fetch(3));
        assert!(model.workbench().active_grid().unwrap().is_resident(3));
    }

    #[test]
    fn scroll_past_resident_emits_fetch_page() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000001".into(),
            identity: "pg".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: Some("connected".into()),
        }));
        model.workbench_mut().open_preview_tab("t");
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.replace_page(
                0,
                vec!["id".into()],
                vec![crate::model::grid::ProjectedCell {
                    text: "1".into(),
                    distinction: crate::model::grid::CellDistinction::Number,
                    byte_len: 1,
                    original_byte_len: None,
                }],
                1,
                GridRowTotal::Unknown,
                1,
                false,
            );
            grid.operation = GridOperationState::Streaming;
            grid.result_token = 42;
            grid.viewport_row = 0;
        }
        let scrolled = update(
            &mut model,
            Message::PointerScrolled {
                target: Some(ShellTarget::Focus(FocusRegion::Content)),
                direction: crate::ScrollDirection::Down,
            },
        );
        let effect = scrolled.effects().next().expect("FetchPage effect");
        match effect {
            Effect::FetchPage {
                result_token,
                start_row,
                ..
            } => {
                assert_eq!(*result_token, 42);
                assert_eq!(*start_row, 1);
            }
            other => panic!("expected FetchPage, got {other:?}"),
        }
    }

    #[test]
    fn grid_stream_complete_marks_completed() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.workbench_mut().context_revision = 1;
        model.workbench_mut().open_preview_tab("t");
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.operation = GridOperationState::Streaming;
            grid.rows_loaded = 500;
        }
        let done = update(
            &mut model,
            Message::Engine(EngineMsg::GridStreamComplete {
                request_token: 1,
                context_revision: 1,
                rows_loaded: 2500,
                truncated: false,
                notice_summary: Some("NOTICE: table-rock-notice".into()),
            }),
        );
        assert!(done.needs_render());
        let grid = model.workbench().active_grid().unwrap();
        assert_eq!(grid.operation, GridOperationState::Completed);
        assert_eq!(grid.rows_loaded, 2500);
        assert_eq!(
            grid.error_label.as_deref(),
            Some("notice: NOTICE: table-rock-notice")
        );
    }

    #[test]
    fn add_filter_and_cycle_sort_rebrowse_with_plan() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000001".into(),
            identity: "pg".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: Some("connected".into()),
        }));
        model.workbench_mut().open_preview_tab("users");
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("public".into());
            grid.base_table = Some("users".into());
            grid.columns = vec!["id".into(), "name".into()];
            grid.row_count = 1;
            grid.cells = vec![
                crate::model::grid::ProjectedCell {
                    text: "1".into(),
                    distinction: crate::model::grid::CellDistinction::Number,
                    byte_len: 1,
                    original_byte_len: None,
                },
                crate::model::grid::ProjectedCell {
                    text: "alice".into(),
                    distinction: crate::model::grid::CellDistinction::Text,
                    byte_len: 5,
                    original_byte_len: None,
                },
            ];
            grid.cursor_col = 1;
            grid.cursor_row = 0;
        }
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::AddFilter);
        let filtered = update(&mut model, Message::Activate);
        match filtered.effects().next() {
            Some(Effect::BrowseTable {
                schema,
                table,
                filters,
                ..
            }) => {
                assert_eq!(schema, "public");
                assert_eq!(table, "users");
                assert_eq!(filters.len(), 1);
                assert_eq!(filters[0].0, "name");
                assert_eq!(filters[0].1, "eq");
                assert_eq!(filters[0].2.as_deref(), Some("alice"));
            }
            other => panic!("expected BrowseTable with filter, got {other:?}"),
        }
        model.set_action(ActionId::CycleSort);
        let sorted = update(&mut model, Message::Activate);
        match sorted.effects().next() {
            Some(Effect::BrowseTable { sort, filters, .. }) => {
                assert!(!sort.is_empty());
                assert_eq!(filters.len(), 1); // filter retained
            }
            other => panic!("expected BrowseTable with sort, got {other:?}"),
        }
        model.set_action(ActionId::ClearFilters);
        let cleared = update(&mut model, Message::Activate);
        match cleared.effects().next() {
            Some(Effect::BrowseTable {
                sort, filters, ..
            }) => {
                assert!(sort.is_empty());
                assert!(filters.is_empty());
            }
            other => panic!("expected cleared BrowseTable, got {other:?}"),
        }
    }

    #[test]
    fn health_failed_auto_reconnects_when_preference_allows() {
        use crate::effect::{ConnectionDraft, EngineKind, PasswordSourceSpec, TlsModeSpec};
        let mut model = Model::default();
        model.reconnect_preference = "BoundedAutomatic".into();
        model.last_connect_draft = Some(ConnectionDraft {
            engine: EngineKind::PostgreSql,
            name: "t".into(),
            group: String::new(),
            environment: String::new(),
            host: "127.0.0.1".into(),
            port: "5432".into(),
            database: "postgres".into(),
            username: "postgres".into(),
            password: String::new(),
            password_source: PasswordSourceSpec::PromptOnConnect,
            tls_mode: TlsModeSpec::Off,
            plaintext_acknowledged: false,
            ssh_host: String::new(),
            ssh_port: String::new(),
            ssh_username: String::new(),
            ssh_password: String::new(),
            ssh_private_key: String::new(),
            ssh_known_hosts_path: String::new(),
            ssh_use_agent: false,
            startup_actions: tablerock_core::StartupActionSet::empty(),
            reconnect_preference: "Manual".into(),
        });
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000088".into(),
            identity: "pg".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: Some("connected".into()),
        }));
        let out = update(
            &mut model,
            Message::Engine(EngineMsg::HealthFailed {
                request_token: 1,
                reason: FailureProjection::Label("connection".into()),
            }),
        );
        match out.effects().next() {
            Some(Effect::ReconnectSession { attempt, .. }) => assert_eq!(*attempt, 0),
            other => panic!("expected auto ReconnectSession, got {other:?}"),
        }
        // Manual preference: no auto reconnect.
        model.reconnect_preference = "Manual".into();
        let manual = update(
            &mut model,
            Message::Engine(EngineMsg::HealthFailed {
                request_token: 2,
                reason: FailureProjection::Label("connection".into()),
            }),
        );
        assert!(manual.effects().next().is_none());
    }

    #[test]
    fn health_tick_probes_only_when_auto_reconnect_and_session() {
        let mut model = Model::default();
        // No session: no effect.
        assert!(update(&mut model, Message::HealthTick).effects().next().is_none());
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000077".into(),
            identity: "pg".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: Some("connected".into()),
        }));
        // Manual: no continuous probe.
        model.reconnect_preference = "Manual".into();
        assert!(update(&mut model, Message::HealthTick).effects().next().is_none());
        // BoundedAutomatic: emit CheckSessionHealth.
        model.reconnect_preference = "BoundedAutomatic".into();
        let out = update(&mut model, Message::HealthTick);
        match out.effects().next() {
            Some(Effect::CheckSessionHealth { session_id_hex, .. }) => {
                assert!(session_id_hex.ends_with("77"));
            }
            other => panic!("expected CheckSessionHealth, got {other:?}"),
        }
        // Skip while reconnecting.
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000077".into(),
            identity: "pg".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: Some("reconnecting attempt 1".into()),
        }));
        assert!(update(&mut model, Message::HealthTick).effects().next().is_none());
    }

    #[test]
    fn reconnecting_message_re_dispatches_effect() {
        use crate::effect::{ConnectionDraft, EngineKind, PasswordSourceSpec, TlsModeSpec};
        let mut model = Model::default();
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000099".into(),
            identity: "pg".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: Some("connected".into()),
        }));
        let draft = ConnectionDraft {
            engine: EngineKind::PostgreSql,
            name: "t".into(),
            group: String::new(),
            environment: String::new(),
            host: "127.0.0.1".into(),
            port: "5432".into(),
            database: "postgres".into(),
            username: "postgres".into(),
            password: String::new(),
            password_source: PasswordSourceSpec::PromptOnConnect,
            tls_mode: TlsModeSpec::Off,
            plaintext_acknowledged: false,
            ssh_host: String::new(),
            ssh_port: String::new(),
            ssh_username: String::new(),
            ssh_password: String::new(),
            ssh_private_key: String::new(),
            ssh_known_hosts_path: String::new(),
            ssh_use_agent: false,
            startup_actions: tablerock_core::StartupActionSet::empty(),
            reconnect_preference: "Manual".into(),
        };
        let out = update(
            &mut model,
            Message::Engine(EngineMsg::Reconnecting {
                request_token: 7,
                attempt: 2,
                next_delay_ms: 4_000,
                draft: draft.clone(),
            }),
        );
        match out.effects().next() {
            Some(Effect::ReconnectSession {
                attempt,
                draft: d,
                request_token,
                ..
            }) => {
                assert_eq!(*request_token, 7);
                assert_eq!(*attempt, 2);
                assert_eq!(d.host, "127.0.0.1");
            }
            other => panic!("expected ReconnectSession re-dispatch, got {other:?}"),
        }
        assert!(model
            .session()
            .and_then(|s| s.status.as_deref())
            .is_some_and(|s| s.contains("reconnecting attempt 2")));
    }

    #[test]
    fn rename_group_dialog_emits_effect() {
        let mut model = Model::default();
        model.set_screen(Screen::Connections);
        model.set_profiles(ProfileListState::Loaded {
            request_token: 1,
            rows: vec![crate::model::profiles::ProfileRowProjection {
                id_hex: "aa".into(),
                name: "local".into(),
                engine_label: "PostgreSQL".into(),
                group: Some("dev".into()),
                favorite: false,
                target_summary: "127.0.0.1".into(),
                environment: None,
                production_warning: false,
                safety_label: "Confirm writes".into(),
                plaintext_secret_warning: false,
                live_state: crate::model::profiles::LiveConnectionState::Disconnected,
            }],
            selected_id: Some("g:dev".into()),
            search: String::new(),
            collapsed: Vec::new(),
        });
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::RenameGroup);
        let ask = update(&mut model, Message::Activate);
        assert!(ask.effects().next().is_none());
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::RenameGroup {
                old_name,
                ..
            }) if old_name == "dev"
        ));
        if let Some(ConfirmDialog::RenameGroup {
            confirm_buffer, ..
        }) = model.confirm_mut()
        {
            *confirm_buffer = "prod".into();
        }
        model.set_action(ActionId::Submit);
        let out = update(&mut model, Message::Activate);
        match out.effects().next() {
            Some(Effect::RenameGroup {
                old_name,
                new_name,
                ..
            }) => {
                assert_eq!(old_name, "dev");
                assert_eq!(new_name, "prod");
            }
            other => panic!("expected RenameGroup, got {other:?}"),
        }
        // Remove on group branch → RemoveGroup confirm.
        model.set_profiles(ProfileListState::Loaded {
            request_token: 2,
            rows: vec![crate::model::profiles::ProfileRowProjection {
                id_hex: "aa".into(),
                name: "local".into(),
                engine_label: "PostgreSQL".into(),
                group: Some("dev".into()),
                favorite: false,
                target_summary: "127.0.0.1".into(),
                environment: None,
                production_warning: false,
                safety_label: "Confirm writes".into(),
                plaintext_secret_warning: false,
                live_state: crate::model::profiles::LiveConnectionState::Disconnected,
            }],
            selected_id: Some("g:dev".into()),
            search: String::new(),
            collapsed: Vec::new(),
        });
        model.set_action(ActionId::Remove);
        let rem = update(&mut model, Message::Activate);
        assert!(rem.effects().next().is_none());
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::RemoveGroup { name }) if name == "dev"
        ));
    }

    #[test]
    fn redis_subscribe_action_opens_confirm_and_emits_effect() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000077".into(),
            identity: "redis".into(),
            temporary: true,
            engine_label: "Redis".into(),
            status: Some("connected".into()),
        }));
        model.workbench_mut().engine_kind = "Redis".into();
        model.workbench_mut().open_sql_tab();
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::RedisSubscribe);
        let ask = update(&mut model, Message::Activate);
        assert!(ask.effects().next().is_none());
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::RedisSubscribe {
                pattern: false,
                ..
            })
        ));
        if let Some(ConfirmDialog::RedisSubscribe {
            confirm_buffer, ..
        }) = model.confirm_mut()
        {
            *confirm_buffer = "news".into();
        }
        model.set_action(ActionId::Submit);
        let out = update(&mut model, Message::Activate);
        match out.effects().next() {
            Some(Effect::RedisSubscribe {
                selector,
                pattern,
                ..
            }) => {
                assert_eq!(selector, "news");
                assert!(!pattern);
            }
            other => panic!("expected RedisSubscribe effect, got {other:?}"),
        }
        // Incremental page keeps streaming; Done completes.
        model.workbench_mut().context_revision = 1;
        let page = update(
            &mut model,
            Message::Engine(EngineMsg::RedisSubscribePage {
                request_token: 1,
                context_revision: 1,
                selector: "news".into(),
                pattern: false,
                lines: vec!["news · hello".into()],
                total_messages: 1,
            }),
        );
        assert!(page.needs_render());
        assert!(model.workbench().inspector.text.contains("hello"));
        assert_eq!(
            model.workbench().active_grid().unwrap().operation,
            GridOperationState::Streaming
        );
        let page2 = update(
            &mut model,
            Message::Engine(EngineMsg::RedisSubscribePage {
                request_token: 1,
                context_revision: 1,
                selector: "news".into(),
                pattern: false,
                lines: vec!["news · world".into()],
                total_messages: 2,
            }),
        );
        assert!(page2.needs_render());
        assert!(model.workbench().inspector.text.contains("hello"));
        assert!(model.workbench().inspector.text.contains("world"));
        let done = update(
            &mut model,
            Message::Engine(EngineMsg::RedisSubscribeDone {
                request_token: 1,
                context_revision: 1,
                selector: "news".into(),
                pattern: false,
                lines: vec!["news · hello".into(), "news · world".into()],
                timed_out: false,
                idle_stop: false,
                cancelled: true,
            }),
        );
        assert!(done.needs_render());
        assert!(model.workbench().inspector.open);
        assert!(model.workbench().inspector.title.contains("cancelled"));
        assert!(model
            .workbench()
            .active_grid()
            .unwrap()
            .operation
            .label()
            .to_ascii_lowercase()
            .contains("cancel"));
    }

    #[test]
    fn backend_signal_permission_denied_marks_grid() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.workbench_mut().context_revision = 2;
        model.workbench_mut().open_preview_tab("activity");
        let out = update(
            &mut model,
            Message::Engine(EngineMsg::BackendSignalFailed {
                request_token: 1,
                context_revision: 2,
                reason: FailureProjection::Label(
                    "permission denied: cannot cancel backends".into(),
                ),
            }),
        );
        assert!(out.needs_render());
        assert!(model
            .workbench()
            .active_grid()
            .and_then(|g| g.error_label.as_deref())
            .is_some_and(|l| l.contains("permission denied") && l.contains("cancel")));
    }

    #[test]
    fn follow_fk_sends_full_row_and_applies_multi_filters() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000055".into(),
            identity: "pg".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: Some("connected".into()),
        }));
        model.workbench_mut().open_preview_tab("orders");
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("public".into());
            grid.base_table = Some("orders".into());
            grid.columns = vec!["tenant_id".into(), "user_id".into(), "total".into()];
            grid.row_count = 1;
            grid.cells = vec![
                crate::model::grid::ProjectedCell {
                    text: "t1".into(),
                    distinction: crate::model::grid::CellDistinction::Text,
                    byte_len: 2,
                    original_byte_len: None,
                },
                crate::model::grid::ProjectedCell {
                    text: "u9".into(),
                    distinction: crate::model::grid::CellDistinction::Text,
                    byte_len: 2,
                    original_byte_len: None,
                },
                crate::model::grid::ProjectedCell {
                    text: "10".into(),
                    distinction: crate::model::grid::CellDistinction::Number,
                    byte_len: 2,
                    original_byte_len: None,
                },
            ];
            grid.cursor_col = 1; // user_id
            grid.cursor_row = 0;
        }
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::FollowForeignKey);
        let out = update(&mut model, Message::Activate);
        match out.effects().next() {
            Some(Effect::LoadForeignKeys {
                local_column,
                row_cells,
                ..
            }) => {
                assert_eq!(local_column, "user_id");
                assert_eq!(row_cells.len(), 3);
                assert!(row_cells.iter().any(|(n, v)| n == "tenant_id" && v == "t1"));
                assert!(row_cells.iter().any(|(n, v)| n == "user_id" && v == "u9"));
            }
            other => panic!("expected LoadForeignKeys with row_cells, got {other:?}"),
        }
        // Engine returns multi-part filters → multiple chips + browse.
        model.workbench_mut().context_revision = 1;
        let applied = update(
            &mut model,
            Message::Engine(EngineMsg::ForeignKeyEdge {
                request_token: 1,
                context_revision: 1,
                foreign_schema: "public".into(),
                foreign_table: "users".into(),
                filters: vec![
                    ("tenant_id".into(), "t1".into()),
                    ("id".into(), "u9".into()),
                ],
            }),
        );
        match applied.effects().next() {
            Some(Effect::BrowseTable {
                table, filters, ..
            }) => {
                assert_eq!(table, "users");
                assert_eq!(filters.len(), 2);
                assert_eq!(filters[0].0, "tenant_id");
                assert_eq!(filters[0].2.as_deref(), Some("t1"));
                assert_eq!(filters[1].0, "id");
                assert_eq!(filters[1].2.as_deref(), Some("u9"));
            }
            other => panic!("expected multi-filter BrowseTable, got {other:?}"),
        }
    }

    #[test]
    fn save_query_and_file_emit_effects() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.workbench_mut().open_sql_tab();
        model
            .workbench_mut()
            .active_editor_mut()
            .unwrap()
            .set_text("SELECT 7");
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::SaveQuery);
        let save_q = update(&mut model, Message::Activate);
        assert!(matches!(
            save_q.effects().next(),
            Some(Effect::SaveNamedQuery {
                statement,
                ..
            }) if statement == "SELECT 7"
        ));
        model.set_action(ActionId::SaveFile);
        let save_f = update(&mut model, Message::Activate);
        assert!(matches!(
            save_f.effects().next(),
            Some(Effect::SaveSqlFile {
                path,
                text,
                ..
            }) if path.ends_with(".sql") && text == "SELECT 7"
        ));
        // Intent requires a profile id.
        model.workbench_mut().profile_id_hex =
            Some("00000000000000010000000000000001".into());
        model.set_action(ActionId::SaveIntent);
        let intent = update(&mut model, Message::Activate);
        assert!(matches!(
            intent.effects().next(),
            Some(Effect::SaveSessionIntent {
                intent_json,
                ..
            }) if intent_json.contains("SELECT 7") && !intent_json.contains("cells")
        ));
    }

    #[test]
    fn history_load_and_restore_into_editor_without_auto_run() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000001".into(),
            identity: "pg".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: Some("connected".into()),
        }));
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::History);
        let load = update(&mut model, Message::Activate);
        assert!(matches!(
            load.effects().next(),
            Some(Effect::LoadHistory {
                limit: 50,
                search: None,
                ..
            })
        ));
        let token = match &model.workbench().history {
            crate::model::history::HistoryPanel::Loading { request_token } => *request_token,
            other => panic!("expected loading, got {other:?}"),
        };
        let filled = update(
            &mut model,
            Message::Engine(EngineMsg::HistoryLoaded {
                request_token: token,
                entries: vec![crate::model::history::HistoryRowProjection {
                    history_id: 1,
                    engine_label: "PostgreSQL".into(),
                    database: "postgres".into(),
                    schema: Some("public".into()),
                    statement_preview: "SELECT 42".into(),
                    outcome: "completed".into(),
                    created_at: "now".into(),
                }],
            }),
        );
        assert!(filled.needs_render());
        model.set_action(ActionId::RestoreHistory);
        let restore = update(&mut model, Message::Activate);
        assert!(restore.effects().next().is_none(), "restore must not auto-execute");
        let editor = model.workbench().active_editor().expect("sql tab");
        assert_eq!(editor.text(), "SELECT 42");
        assert!(!model.workbench().history.is_open());
    }

    #[test]
    fn run_sql_uses_selection_else_current_statement() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000001".into(),
            identity: "pg".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: Some("connected".into()),
        }));
        model.workbench_mut().open_sql_tab();
        {
            let editor = model.workbench_mut().active_editor_mut().unwrap();
            editor.set_text("SELECT 1; SELECT 2");
            let spans = editor.spans().to_vec();
            editor.set_selection(spans[1].start, spans[1].end);
        }
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::RunSql);
        let run = update(&mut model, Message::Activate);
        let effect = run.effects().next().expect("ExecuteSql");
        match effect {
            Effect::ExecuteSql { statement, .. } => {
                assert!(
                    statement.contains("SELECT 2") || statement.trim_start().starts_with("SELECT 2"),
                    "{statement}"
                );
            }
            other => panic!("expected ExecuteSql, got {other:?}"),
        }
        // Clear selection → current statement under cursor (first).
        {
            let editor = model.workbench_mut().active_editor_mut().unwrap();
            editor.clear_selection();
            editor.set_cursor(0);
        }
        model.set_action(ActionId::RunSql);
        let run2 = update(&mut model, Message::Activate);
        match run2.effects().next().expect("ExecuteSql") {
            Effect::ExecuteSql { statement, .. } => {
                assert!(statement.starts_with("SELECT 1"), "{statement}");
            }
            other => panic!("expected ExecuteSql, got {other:?}"),
        }
    }

    #[test]
    fn cancel_dispatch_and_outcome_labels_are_distinct() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.workbench_mut().open_preview_tab("t");
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.operation = GridOperationState::Streaming;
        }
        let requested = update(
            &mut model,
            Message::Engine(EngineMsg::GridCancelDispatched {
                request_token: 1,
                dispatch: "request_sent".into(),
            }),
        );
        assert!(requested.needs_render());
        assert_eq!(
            model.workbench().active_grid().unwrap().operation,
            GridOperationState::CancelRequested
        );
        assert_eq!(
            model.workbench().active_grid().unwrap().operation.label(),
            "cancel requested"
        );
        // Transport failure → distinct unknown cancel state.
        let unknown = update(
            &mut model,
            Message::Engine(EngineMsg::GridCancelDispatched {
                request_token: 2,
                dispatch: "transport_failed".into(),
            }),
        );
        assert!(unknown.needs_render());
        assert_eq!(
            model.workbench().active_grid().unwrap().operation,
            GridOperationState::CancelUnknown
        );
        assert_eq!(
            model.workbench().active_grid().unwrap().operation.label(),
            "cancel unknown"
        );
        let observed = update(
            &mut model,
            Message::Engine(EngineMsg::GridCancelled {
                request_token: 1,
                label: "server confirmed cancelled".into(),
            }),
        );
        assert!(observed.needs_render());
        let grid = model.workbench().active_grid().unwrap();
        assert_eq!(
            grid.operation,
            GridOperationState::ServerConfirmedCancelled
        );
        assert_eq!(grid.operation.label(), "server confirmed cancelled");
        assert_eq!(
            grid.error_label.as_deref(),
            Some("server confirmed cancelled")
        );
        // Client-stopped dispatch.
        let client = update(
            &mut model,
            Message::Engine(EngineMsg::GridCancelDispatched {
                request_token: 3,
                dispatch: "prevented".into(),
            }),
        );
        assert!(client.needs_render());
        assert_eq!(
            model.workbench().active_grid().unwrap().operation.label(),
            "client stopped"
        );
    }

    #[test]
    fn grid_page_fills_active_tab_and_rejects_stale_context() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.workbench_mut().context_revision = 2;
        model.workbench_mut().open_preview_tab("users");
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.operation = GridOperationState::Running;
            grid.base_schema = Some("public".into());
            grid.base_table = Some("users".into());
        }
        let stale = update(
            &mut model,
            Message::Engine(EngineMsg::GridPage {
                request_token: 1,
                context_revision: 1,
                start_row: 0,
                columns: vec!["id".into()],
                cells: vec![crate::model::grid::ProjectedCell {
                    text: "1".into(),
                    distinction: crate::model::grid::CellDistinction::Number,
                    byte_len: 1,
                    original_byte_len: None,
                }],
                row_count: 1,
                totals_exact: Some(1),
                totals_estimated: None,
                bytes: 8,
                truncated: false,
                complete: true,
                identity_columns: None,
                server_query_id: None,
                server_progress: None,
            }),
        );
        assert!(!stale.needs_render());
        let ok = update(
            &mut model,
            Message::Engine(EngineMsg::GridPage {
                request_token: 1,
                context_revision: 2,
                start_row: 0,
                columns: vec!["id".into()],
                cells: vec![crate::model::grid::ProjectedCell {
                    text: "1".into(),
                    distinction: crate::model::grid::CellDistinction::Number,
                    byte_len: 1,
                    original_byte_len: None,
                }],
                row_count: 1,
                totals_exact: Some(1),
                totals_estimated: None,
                bytes: 8,
                truncated: false,
                complete: true,
                identity_columns: Some(vec!["id".into()]),
                server_query_id: Some("tr-1".into()),
                server_progress: Some("read 10 rows · 128 B".into()),
            }),
        );
        assert!(ok.needs_render());
        let grid = model.workbench().active_grid().unwrap();
        assert_eq!(grid.row_count, 1);
        assert_eq!(grid.columns, ["id"]);
        assert_eq!(grid.operation, GridOperationState::Completed);
        assert!(grid.is_resident(0));
        assert_eq!(grid.identity_columns, vec!["id".to_owned()]);
        assert!(grid.editability.is_editable());
        assert_eq!(grid.server_query_id.as_deref(), Some("tr-1"));
        assert_eq!(
            grid.server_progress.as_deref(),
            Some("read 10 rows · 128 B")
        );
        assert!(grid.status_line().contains("qid tr-1"));
        assert!(grid.status_line().contains("read 10 rows"));
    }

    #[test]
    fn database_switch_bumps_revision_and_reloads_catalog() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000001".into(),
            identity: "pg".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: Some("connected".into()),
        }));
        model.workbench_mut().context_revision = 3;
        model.workbench_mut().context.database = "postgres".into();
        model.workbench_mut().engine_kind = "PostgreSQL".into();
        model.workbench_mut().catalog = CatalogModel::Loaded {
            request_token: 1,
            context_revision: 3,
            nodes: vec![
                crate::model::catalog::CatalogNodeProjection {
                    id: "postgres".into(),
                    label: "postgres".into(),
                    kind_label: "database".into(),
                    depth: 0,
                    branch: true,
                    expanded: false,
                    status: CatalogNodeStatus::Ready,
                },
                crate::model::catalog::CatalogNodeProjection {
                    id: "app".into(),
                    label: "app".into(),
                    kind_label: "database".into(),
                    depth: 0,
                    branch: true,
                    expanded: false,
                    status: CatalogNodeStatus::Ready,
                },
            ],
            selected_id: None,
            filter: String::new(),
            truncated: false,
        };
        model.set_action(ActionId::NextDatabase);
        for _ in 0..4 {
            let _ = update(&mut model, Message::FocusNext);
        }
        model.set_action(ActionId::NextDatabase);
        let result = update(&mut model, Message::Activate);
        assert_eq!(model.workbench().context.database, "app");
        assert_eq!(model.workbench().context_revision, 4);
        assert!(matches!(
            result.effects().next(),
            Some(Effect::LoadCatalog {
                context_revision: 4,
                ..
            })
        ));
        // Stale catalog completion for rev 3 is ignored.
        let stale = update(
            &mut model,
            Message::Engine(EngineMsg::CatalogLoaded {
                request_token: 1,
                context_revision: 3,
                parent_id: None,
                nodes: vec![],
                truncated: false,
            }),
        );
        assert!(!stale.needs_render());
        assert!(matches!(
            model.workbench().catalog,
            CatalogModel::Loading {
                context_revision: 4,
                ..
            }
        ));
    }

    #[test]
    fn apply_requires_review_token_and_review_registers_specs() {
        use crate::model::mutation_draft::{DraftLocatorField, StagedCellEdit};
        use tablerock_core::ProfileSafetyMode;

        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "00000000000000010000000000000001".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("public".into());
            grid.base_table = Some("users".into());
            grid.identity_columns = vec!["id".into()];
            grid.recompute_editability(ProfileSafetyMode::ConfirmWrites, false);
            assert!(grid.drafts.stage_cell_edit(StagedCellEdit {
                abs_row: 0,
                column: "name".into(),
                original_text: "a".into(),
                staged_text: "b".into(),
                locator: vec![DraftLocatorField {
                    column: "id".into(),
                    original_text: "1".into(),
                }],
            }));
        }
        // Apply without review → no effect, status message.
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::ApplyMutations);
        let blocked = update(&mut model, Message::Activate);
        assert!(blocked.effects().next().is_none());
        assert!(model
            .workbench()
            .active_grid()
            .unwrap()
            .error_label
            .as_deref()
            .unwrap_or("")
            .contains("review required"));
        // Review dispatches ReviewMutations with change specs (registry path).
        model.set_action(ActionId::ReviewMutations);
        let reviewed = update(&mut model, Message::Activate);
        assert!(matches!(
            reviewed.effects().next(),
            Some(Effect::ReviewMutations { changes, .. }) if !changes.is_empty()
        ));
        // Simulate ready token then apply is handle-only.
        let rev = model.workbench().context_revision;
        let _ = update(
            &mut model,
            Message::Engine(EngineMsg::MutationReviewReady {
                request_token: 1,
                context_revision: rev,
                review_token_hex: "00000000000000010000000000000002".into(),
                expires_at_ms: 9_999_999,
                lines: vec!["1: UPDATE …".into()],
            }),
        );
        assert_eq!(
            model.workbench().pending_review_token_hex.as_deref(),
            Some("00000000000000010000000000000002")
        );
        model.set_action(ActionId::ApplyMutations);
        let apply = update(&mut model, Message::Activate);
        assert!(matches!(
            apply.effects().next(),
            Some(Effect::ApplyMutations {
                review_token_hex,
                ..
            }) if review_token_hex == "00000000000000010000000000000002"
        ));
        // Expiry/re-review clears handle.
        let rev = model.workbench().context_revision;
        let _ = update(
            &mut model,
            Message::Engine(EngineMsg::MutationFailed {
                request_token: 2,
                context_revision: rev,
                reason: FailureProjection::Label(
                    "authorize failed (Expired); re-review required".into(),
                ),
                needs_re_review: true,
            }),
        );
        assert!(model.workbench().pending_review_token_hex.is_none());
    }

    #[test]
    fn truncate_confirm_requires_exact_table_name() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "01".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("public".into());
            grid.base_table = Some("users".into());
        }
        // Drive the action match path directly via Actions focus.
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::TruncateTable);
        let _ = update(&mut model, Message::Activate);
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::TruncateTable { table, .. }) if table == "users"
        ));
        // Wrong name: no effect.
        model.set_action(ActionId::Submit);
        let wrong = update(&mut model, Message::Activate);
        assert!(wrong.effects().next().is_none());
        assert!(model.confirm().is_some());
        // Paste exact name then submit.
        let _ = update(&mut model, Message::Paste(PasteText::bounded("users".into())));
        model.set_action(ActionId::Submit);
        let ok = update(&mut model, Message::Activate);
        assert!(matches!(
            ok.effects().next(),
            Some(Effect::ExecuteTableOp {
                op,
                table,
                new_table,
                ..
            }) if op == "truncate" && table == "users" && new_table.is_empty()
        ));
        assert!(model.confirm().is_none());
    }

    #[test]
    fn rename_table_confirm_emits_execute_table_op() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "01".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("public".into());
            grid.base_table = Some("users".into());
        }
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::RenameTable);
        let _ = update(&mut model, Message::Activate);
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::RenameTable { table, .. }) if table == "users"
        ));
        // Empty / same name: no effect.
        model.set_action(ActionId::Submit);
        let empty = update(&mut model, Message::Activate);
        assert!(empty.effects().next().is_none());
        assert!(model.confirm().is_some());
        // Same as current name rejected.
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("users".into())),
        );
        model.set_action(ActionId::Submit);
        let same = update(&mut model, Message::Activate);
        assert!(same.effects().next().is_none());
        // New name dispatches rename.
        if let Some(ConfirmDialog::RenameTable {
            confirm_buffer, ..
        }) = model.confirm_mut()
        {
            *confirm_buffer = "users_v2".into();
        }
        model.set_action(ActionId::Submit);
        let ok = update(&mut model, Message::Activate);
        assert!(matches!(
            ok.effects().next(),
            Some(Effect::ExecuteTableOp {
                op,
                table,
                new_table,
                schema,
                ..
            }) if op == "rename"
                && table == "users"
                && new_table == "users_v2"
                && schema == "public"
        ));
        assert!(model.confirm().is_none());
    }

    #[test]
    fn copy_structure_ddl_emits_clipboard_effect() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "01".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        model.workbench_mut().inspector = crate::model::inspector::InspectorModel {
            open: true,
            title: "public.users structure".into(),
            kind_label: "structure".into(),
            text: "\
-- columns --
id integer NOT NULL
name text NULL
-- constraints --
PRIMARY KEY users_pkey: PRIMARY KEY (id)
"
            .into(),
            hex: String::new(),
            byte_len: 64,
            original_byte_len: None,
            stale: false,
            structure_schema: Some("public".into()),
            structure_table: Some("users".into()),
        };
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::CopyStructureDdl);
        let out = update(&mut model, Message::Activate);
        match out.effects().next() {
            Some(Effect::CopyToClipboard { text, .. }) => {
                assert!(text.contains("CREATE TABLE \"public\".\"users\""));
                assert!(text.contains("id integer NOT NULL"));
                assert!(text.contains("CONSTRAINT \"users_pkey\""));
            }
            other => panic!("expected CopyToClipboard, got {other:?}"),
        }
    }

    #[test]
    fn vacuum_table_confirm_requires_exact_name() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "01".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("public".into());
            grid.base_table = Some("users".into());
        }
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::VacuumTable);
        let _ = update(&mut model, Message::Activate);
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::VacuumTable { table, .. }) if table == "users"
        ));
        model.set_action(ActionId::Submit);
        let wrong = update(&mut model, Message::Activate);
        assert!(wrong.effects().next().is_none());
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("users".into())),
        );
        model.set_action(ActionId::Submit);
        let ok = update(&mut model, Message::Activate);
        assert!(matches!(
            ok.effects().next(),
            Some(Effect::ExecuteTableOp {
                op,
                table,
                new_table,
                ..
            }) if op == "vacuum" && table == "users" && new_table.is_empty()
        ));
        assert!(model.confirm().is_none());
    }

    #[test]
    fn analyze_table_confirm_requires_exact_name() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "01".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("public".into());
            grid.base_table = Some("orders".into());
        }
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::AnalyzeTable);
        let _ = update(&mut model, Message::Activate);
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::AnalyzeTable { table, .. }) if table == "orders"
        ));
        model.set_action(ActionId::Submit);
        assert!(update(&mut model, Message::Activate).effects().next().is_none());
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("orders".into())),
        );
        model.set_action(ActionId::Submit);
        let ok = update(&mut model, Message::Activate);
        assert!(matches!(
            ok.effects().next(),
            Some(Effect::ExecuteTableOp { op, table, .. }) if op == "analyze" && table == "orders"
        ));
    }

    #[test]
    fn optimize_table_confirm_requires_exact_name() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "01".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "ClickHouse".into(),
            status: None,
        }));
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("default".into());
            grid.base_table = Some("events".into());
        }
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::OptimizeTable);
        let _ = update(&mut model, Message::Activate);
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::OptimizeTable { table, schema, .. })
                if table == "events" && schema == "default"
        ));
        model.set_action(ActionId::Submit);
        assert!(update(&mut model, Message::Activate).effects().next().is_none());
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("events".into())),
        );
        model.set_action(ActionId::Submit);
        let ok = update(&mut model, Message::Activate);
        assert!(matches!(
            ok.effects().next(),
            Some(Effect::ExecuteTableOp {
                op,
                table,
                schema,
                ..
            }) if op == "optimize" && table == "events" && schema == "default"
        ));
    }

    #[test]
    fn ddl_add_column_review_emits_execute_ddl_plan() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "aabb".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("public".into());
            grid.base_table = Some("users".into());
        }
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::DdlAddColumn);
        let _ = update(&mut model, Message::Activate);
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::DdlReview { kind, table, .. })
                if kind == "add_column" && table == "users"
        ));
        // Incomplete buffer: no effect.
        model.set_action(ActionId::Submit);
        let incomplete = update(&mut model, Message::Activate);
        assert!(incomplete.effects().next().is_none());
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("note text".into())),
        );
        model.set_action(ActionId::Submit);
        let ok = update(&mut model, Message::Activate);
        assert!(matches!(
            ok.effects().next(),
            Some(Effect::ExecuteDdlPlan {
                kind,
                object_name,
                type_text,
                schema,
                table,
                ..
            }) if kind == "add_column"
                && object_name == "note"
                && type_text == "text"
                && schema == "public"
                && table == "users"
        ));
        assert!(model.confirm().is_none());
    }

    #[test]
    fn ddl_add_constraint_review_emits_execute_ddl_plan() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "aabb".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("public".into());
            grid.base_table = Some("users".into());
        }
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::DdlAddConstraint);
        let _ = update(&mut model, Message::Activate);
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("users_email_uq UNIQUE (email)".into())),
        );
        model.set_action(ActionId::Submit);
        let ok = update(&mut model, Message::Activate);
        assert!(matches!(
            ok.effects().next(),
            Some(Effect::ExecuteDdlPlan {
                kind,
                object_name,
                type_text,
                ..
            }) if kind == "add_constraint"
                && object_name == "users_email_uq"
                && type_text == "UNIQUE (email)"
        ));
    }

    #[test]
    fn ddl_drop_column_review_emits_execute_ddl_plan() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "aabb".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("public".into());
            grid.base_table = Some("users".into());
        }
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::DdlDropColumn);
        let _ = update(&mut model, Message::Activate);
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::DdlReview { kind, .. }) if kind == "drop_column"
        ));
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("email".into())),
        );
        model.set_action(ActionId::Submit);
        let ok = update(&mut model, Message::Activate);
        assert!(matches!(
            ok.effects().next(),
            Some(Effect::ExecuteDdlPlan {
                kind,
                object_name,
                type_text,
                ..
            }) if kind == "drop_column" && object_name == "email" && type_text.is_empty()
        ));
    }

    #[test]
    fn format_sql_action_uppercases_keywords() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "aabb".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        model.workbench_mut().open_sql_tab();
        model
            .workbench_mut()
            .active_editor_mut()
            .unwrap()
            .set_text("select  *  from  t");
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::FormatSql);
        let _ = update(&mut model, Message::Activate);
        assert_eq!(
            model.workbench().active_editor().unwrap().text(),
            "SELECT * FROM t"
        );
    }

    #[test]
    fn find_replace_action_rewrites_editor_text() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "aabb".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        model.workbench_mut().open_sql_tab();
        model
            .workbench_mut()
            .active_editor_mut()
            .unwrap()
            .set_text("SELECT foo FROM t");
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::FindReplace);
        let _ = update(&mut model, Message::Activate);
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("foo=>bar=>all".into())),
        );
        model.set_action(ActionId::Submit);
        let _ = update(&mut model, Message::Activate);
        assert_eq!(
            model.workbench().active_editor().unwrap().text(),
            "SELECT bar FROM t"
        );
    }

    #[test]
    fn run_sql_multi_statement_selection_emits_script_effect() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "aabb".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        model.workbench_mut().open_sql_tab();
        let ed = model.workbench_mut().active_editor_mut().unwrap();
        ed.set_text("SELECT 1;\nSELECT 2;");
        // Select entire buffer so ≥2 spans are covered.
        let len = ed.text().len();
        ed.set_selection(0, len);
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::RunSql);
        let out = update(&mut model, Message::Activate);
        assert!(matches!(
            out.effects().next(),
            Some(Effect::ExecuteSqlScript {
                statements,
                session_id_hex,
                ..
            }) if session_id_hex == "aabb" && statements.len() == 2
        ));
        assert_eq!(model.workbench().result_sections.sections.len(), 2);
    }

    #[test]
    fn run_script_action_runs_entire_buffer_without_prior_selection() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "ccdd".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        model.workbench_mut().open_sql_tab();
        let ed = model.workbench_mut().active_editor_mut().unwrap();
        ed.set_text("SELECT 1;\nSELECT 2;\nSELECT 3;");
        // Cursor only — no multi-span selection.
        ed.set_cursor(0);
        ed.clear_selection();
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::RunScript);
        let out = update(&mut model, Message::Activate);
        assert!(matches!(
            out.effects().next(),
            Some(Effect::ExecuteSqlScript {
                statements,
                session_id_hex,
                ..
            }) if session_id_hex == "ccdd" && statements.len() == 3
        ));
        assert_eq!(model.workbench().result_sections.sections.len(), 3);
    }

    #[test]
    fn run_sql_with_named_params_opens_bind_dialog() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "aabb".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        model.workbench_mut().open_sql_tab();
        model
            .workbench_mut()
            .active_editor_mut()
            .unwrap()
            .set_text("SELECT :id, :name");
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::RunSql);
        let out = update(&mut model, Message::Activate);
        assert!(out.effects().next().is_none());
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::BindParams { names, .. })
                if names.contains(&"id".into()) && names.contains(&"name".into())
        ));
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("id=1; name=alice".into())),
        );
        model.set_action(ActionId::Submit);
        let run = update(&mut model, Message::Activate);
        assert!(matches!(
            run.effects().next(),
            Some(Effect::ExecuteSql {
                statement,
                parameters,
                ..
            }) if statement.contains("$1")
                && statement.contains("$2")
                && !statement.contains(":id")
                && *parameters == ["1".to_owned(), "alice".to_owned()]
        ));
        assert!(model.confirm().is_none());
    }

    #[test]
    fn quick_switch_selects_tab_by_title_substring() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "aabb".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        model.workbench_mut().open_sql_tab();
        model.workbench_mut().tabs[1].title = "report.sql".into();
        model.workbench_mut().open_sql_tab();
        model.workbench_mut().tabs[2].title = "orders".into();
        assert_eq!(model.workbench().selected_tab, 2);
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::QuickSwitch);
        let _ = update(&mut model, Message::Activate);
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("report".into())),
        );
        model.set_action(ActionId::Submit);
        let _ = update(&mut model, Message::Activate);
        assert_eq!(model.workbench().selected_tab, 1);
        assert_eq!(model.workbench().tabs[1].title, "report.sql");
    }

    #[test]
    fn quick_switch_ranks_profiles_on_connections() {
        use crate::model::profiles::{
            LiveConnectionState, ProfileListState, ProfileRowProjection,
        };
        let mut model = Model::default();
        model.set_screen(Screen::Connections);
        model.set_profiles(ProfileListState::Loaded {
            request_token: 1,
            rows: vec![
                ProfileRowProjection {
                    id_hex: "1111".into(),
                    name: "staging".into(),
                    engine_label: "PostgreSQL".into(),
                    group: None,
                    favorite: false,
                    target_summary: "s.example:5432/app".into(),
                    environment: None,
                    production_warning: false,
                    safety_label: "Confirm writes".into(),
                    plaintext_secret_warning: false,
                    live_state: LiveConnectionState::Disconnected,
                },
                ProfileRowProjection {
                    id_hex: "2222".into(),
                    name: "prod-app".into(),
                    engine_label: "PostgreSQL".into(),
                    group: None,
                    favorite: true,
                    target_summary: "p.example:5432/app".into(),
                    environment: Some("production".into()),
                    production_warning: true,
                    safety_label: "Confirm writes".into(),
                    plaintext_secret_warning: false,
                    live_state: LiveConnectionState::Disconnected,
                },
            ],
            selected_id: Some("1111".into()),
            search: String::new(),
            collapsed: Vec::new(),
        });
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::QuickSwitch);
        let _ = update(&mut model, Message::Activate);
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("prod".into())),
        );
        model.set_action(ActionId::Submit);
        let _ = update(&mut model, Message::Activate);
        assert!(matches!(
            model.profiles(),
            ProfileListState::Loaded {
                selected_id: Some(id),
                ..
            } if id == "2222"
        ));
    }

    #[test]
    fn quick_switch_loads_saved_query_by_name() {
        use crate::model::saved_query::{SavedQueryPanel, SavedQueryRow};
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "aabb".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        model.workbench_mut().saved_queries = SavedQueryPanel::Open {
            request_token: 1,
            entries: vec![SavedQueryRow {
                query_id: 42,
                name: "weekly-report".into(),
                engine_label: "PostgreSQL".into(),
                statement_preview: "select 1".into(),
            }],
            selected: 0,
        };
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::QuickSwitch);
        let _ = update(&mut model, Message::Activate);
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("weekly".into())),
        );
        model.set_action(ActionId::Submit);
        let out = update(&mut model, Message::Activate);
        assert!(matches!(
            out.effects().next(),
            Some(Effect::LoadNamedQuery {
                query_id: 42,
                ..
            })
        ));
    }

    #[test]
    fn explain_wraps_editor_sql_for_postgres() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "aabb".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        model.workbench_mut().open_sql_tab();
        model
            .workbench_mut()
            .active_editor_mut()
            .unwrap()
            .set_text("select 1");
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::Explain);
        let out = update(&mut model, Message::Activate);
        assert!(matches!(
            out.effects().next(),
            Some(Effect::ExecuteSql {
                statement,
                session_id_hex,
                ..
            }) if session_id_hex == "aabb"
                && statement.starts_with("EXPLAIN (FORMAT TEXT)")
                && statement.contains("select 1")
        ));
    }

    #[test]
    fn explain_redis_is_unsupported() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "ccdd".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "Redis".into(),
            status: None,
        }));
        model.workbench_mut().open_sql_tab();
        model
            .workbench_mut()
            .active_editor_mut()
            .unwrap()
            .set_text("GET k");
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::Explain);
        let out = update(&mut model, Message::Activate);
        assert!(out.effects().next().is_none());
        assert!(model
            .workbench()
            .active_grid()
            .and_then(|g| g.error_label.as_ref())
            .is_some_and(|e| e.contains("unsupported")));
    }

    #[test]
    fn open_external_url_requires_open_then_connects_temporary() {
        let mut model = Model::default();
        model.set_screen(Screen::Connections);
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::OpenExternalUrl);
        let _ = update(&mut model, Message::Activate);
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::OpenExternalUrl { summary, .. }) if summary.is_empty()
        ));
        // Phase 1: paste URL
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded(
                "postgres://alice:s3cret@db.example:5432/app".into(),
            )),
        );
        model.set_action(ActionId::Submit);
        let _ = update(&mut model, Message::Activate);
        match model.confirm() {
            Some(ConfirmDialog::OpenExternalUrl {
                summary,
                url,
                matched_profile_id_hex,
                ..
            }) => {
                assert!(summary.contains("PostgreSQL"));
                assert!(summary.contains("password=present"));
                assert!(summary.contains("temporary session"));
                assert!(!summary.contains("s3cret"));
                assert!(url.contains("s3cret")); // retained for re-parse only
                assert!(matched_profile_id_hex.is_none());
            }
            other => panic!("expected OpenExternalUrl summary, got {other:?}"),
        }
        // Phase 2: paste OPEN → temporary connect effect
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("OPEN".into())),
        );
        model.set_action(ActionId::Submit);
        let out = update(&mut model, Message::Activate);
        assert!(matches!(
            out.effects().next(),
            Some(Effect::ConnectSession {
                temporary: true,
                draft,
                ..
            }) if draft.host == "db.example"
                && draft.database == "app"
                && draft.username == "alice"
                && draft.password == "s3cret"
        ));
        assert!(model.confirm().is_none());
    }

    #[test]
    fn open_external_url_matches_saved_profile() {
        use crate::model::profiles::{
            LiveConnectionState, ProfileListState, ProfileRowProjection,
        };
        let mut model = Model::default();
        model.set_screen(Screen::Connections);
        model.set_profiles(ProfileListState::Loaded {
            request_token: 1,
            rows: vec![ProfileRowProjection {
                id_hex: "aabbccdd".into(),
                name: "prod-app".into(),
                engine_label: "PostgreSQL".into(),
                group: None,
                favorite: false,
                target_summary: "db.example:5432/app".into(),
                environment: Some("production".into()),
                production_warning: true,
                safety_label: "Confirm writes".into(),
                plaintext_secret_warning: false,
                live_state: LiveConnectionState::Disconnected,
            }],
            selected_id: None,
            search: String::new(),
            collapsed: Vec::new(),
        });
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::OpenExternalUrl);
        let _ = update(&mut model, Message::Activate);
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded(
                "postgres://alice@db.example:5432/app".into(),
            )),
        );
        model.set_action(ActionId::Submit);
        let _ = update(&mut model, Message::Activate);
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::OpenExternalUrl {
                matched_profile_id_hex: Some(id),
                summary,
                ..
            }) if id == "aabbccdd" && summary.contains("prod-app")
        ));
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("OPEN".into())),
        );
        model.set_action(ActionId::Submit);
        let out = update(&mut model, Message::Activate);
        assert!(matches!(
            out.effects().next(),
            Some(Effect::ConnectProfile {
                profile_id_hex,
                ..
            }) if profile_id_hex == "aabbccdd"
        ));
    }

    #[test]
    fn open_external_url_rejects_hostile_scheme() {
        let mut model = Model::default();
        model.set_screen(Screen::Connections);
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::OpenExternalUrl);
        let _ = update(&mut model, Message::Activate);
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("javascript://alert(1)".into())),
        );
        model.set_action(ActionId::Submit);
        let _ = update(&mut model, Message::Activate);
        assert!(model.confirm().is_none());
        assert!(model
            .editor()
            .validation_error
            .as_deref()
            .is_some_and(|e| e.contains("hostile") || e.contains("unsupported")));
    }

    #[test]
    fn import_url_paste_applies_to_editor() {
        let mut model = Model::default();
        model.set_screen(Screen::Connections);
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::ImportUrl);
        let _ = update(&mut model, Message::Activate);
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::ImportUrl { .. })
        ));
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded(
                "redis://:hunter2@127.0.0.1:6380/2".into(),
            )),
        );
        model.set_action(ActionId::Submit);
        let _ = update(&mut model, Message::Activate);
        assert!(model.confirm().is_none());
        assert_eq!(model.screen(), Screen::Editor);
        assert_eq!(model.editor().engine, crate::effect::EngineKind::Redis);
        assert_eq!(model.editor().host, "127.0.0.1");
        assert_eq!(model.editor().port, "6380");
        assert_eq!(model.editor().database, "2");
        assert_eq!(model.editor().password, "hunter2");
    }

    #[test]
    fn pg_dump_action_opens_confirm_and_emits_run_effect() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "aabb".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        model.editor_mut().host = "127.0.0.1".into();
        model.editor_mut().port = "5432".into();
        model.editor_mut().database = "postgres".into();
        model.editor_mut().username = "postgres".into();
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::PgDump);
        let _ = update(&mut model, Message::Activate);
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::PgTool { kind, .. }) if kind == "dump"
        ));
        let _ = update(
            &mut model,
            Message::Paste(PasteText::bounded("/tmp/t.dump".into())),
        );
        model.set_action(ActionId::Submit);
        let out = update(&mut model, Message::Activate);
        assert!(matches!(
            out.effects().next(),
            Some(Effect::RunPgDump {
                host,
                path,
                database,
                ..
            }) if host == "127.0.0.1" && path == "/tmp/t.dump" && database == "postgres"
        ));
    }

    #[test]
    fn structure_panel_target_enables_ddl_without_grid_base() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "aabb".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        // No grid base table — only structure inspector target.
        model.workbench_mut().inspector = crate::model::inspector::InspectorModel {
            open: true,
            title: "public.orders structure".into(),
            kind_label: "structure".into(),
            text: "id int\n--- quick actions ---".into(),
            hex: String::new(),
            byte_len: 0,
            original_byte_len: None,
            stale: false,
            structure_schema: Some("public".into()),
            structure_table: Some("orders".into()),
        };
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::DdlAddColumn);
        let _ = update(&mut model, Message::Activate);
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::DdlReview {
                kind,
                schema,
                table,
                ..
            }) if kind == "add_column" && schema == "public" && table == "orders"
        ));
    }

    #[test]
    fn connect_with_startup_pending_opens_review_dialog() {
        let mut model = Model::default();
        model.set_screen(Screen::Editor);
        let _ = update(
            &mut model,
            Message::Engine(EngineMsg::ConnectOk {
                request_token: 1,
                session_id_hex: "00000000000000010000000000000002".into(),
                identity: "PostgreSQL 17".into(),
                temporary: true,
                engine_label: "PostgreSQL".into(),
                profile_id_hex: None,
                startup_summary: Some("startup 1ok/1skip/0fail/0timeout".into()),
                startup_pending: vec![("write".into(), "SET search_path TO app".into())],
                reconnect_preference: Some("BoundedAutomatic".into()),
            }),
        );
        assert_eq!(model.screen(), Screen::Workbench);
        assert_eq!(model.reconnect_preference, "BoundedAutomatic");
        assert!(matches!(
            model.confirm(),
            Some(ConfirmDialog::StartupReview { items, .. }) if items.len() == 1
        ));
        let _ = update(&mut model, Message::Paste(PasteText::bounded("RUN".into())));
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::Submit);
        let out = update(&mut model, Message::Activate);
        assert!(matches!(
            out.effects().next(),
            Some(Effect::ExecuteStartupReviewed {
                items,
                ..
            }) if items.len() == 1 && items[0].0 == "write"
        ));
        assert!(model.confirm().is_none());
    }

    #[test]
    fn show_roles_emits_load_roles_with_base_table() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "aabb".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("public".into());
            grid.base_table = Some("users".into());
        }
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::ShowRoles);
        let out = update(&mut model, Message::Activate);
        assert!(matches!(
            out.effects().next(),
            Some(Effect::LoadRoles {
                session_id_hex,
                schema: Some(schema),
                table: Some(table),
                ..
            }) if session_id_hex == "aabb" && schema == "public" && table == "users"
        ));
        let context_revision = model.workbench().context_revision;
        let snap = update(
            &mut model,
            Message::Engine(EngineMsg::RolesSnapshot {
                request_token: 1,
                context_revision,
                lines: vec![
                    "member: alice".into(),
                    "effective: alice, parent".into(),
                    "self-cycle: no".into(),
                ],
            }),
        );
        assert!(snap.render);
        assert!(model.workbench().inspector.open);
        assert_eq!(model.workbench().inspector.title, "roles");
        assert!(model.workbench().inspector.text.contains("effective:"));
    }

    #[test]
    fn export_stream_emits_export_stream_query_from_editor_sql() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "aabb".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        model.workbench_mut().open_sql_tab();
        model
            .workbench_mut()
            .active_editor_mut()
            .unwrap()
            .set_text("select 1 as n");
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::ExportStreamCsv);
        let out = update(&mut model, Message::Activate);
        assert!(matches!(
            out.effects().next(),
            Some(Effect::ExportStreamQuery {
                session_id_hex,
                statement,
                format,
                path,
                ..
            }) if session_id_hex == "aabb"
                && statement.contains("select 1")
                && format == "csv"
                && path == "export-stream.csv"
        ));
    }

    #[test]
    fn import_csv_emits_import_apply_for_base_table() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "ccdd".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        model.workbench_mut().context.database = "postgres".into();
        if let Some(grid) = model.workbench_mut().active_grid_mut() {
            grid.base_schema = Some("public".into());
            grid.base_table = Some("users".into());
        }
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::ImportCsv);
        let out = update(&mut model, Message::Activate);
        assert!(matches!(
            out.effects().next(),
            Some(Effect::ImportCsvApply {
                session_id_hex,
                database,
                schema,
                table,
                path,
                ..
            }) if session_id_hex == "ccdd"
                && database == "postgres"
                && schema == "public"
                && table == "users"
                && path == "import.csv"
        ));
    }

    #[test]
    fn import_csv_without_base_table_marks_failed() {
        let mut model = Model::default();
        model.set_screen(Screen::Workbench);
        model.set_session(Some(SessionFacts {
            session_id_hex: "eeff".into(),
            identity: "local".into(),
            temporary: true,
            engine_label: "PostgreSQL".into(),
            status: None,
        }));
        let _ = model.request_focus(FocusRegion::Actions);
        model.set_action(ActionId::ImportCsv);
        let out = update(&mut model, Message::Activate);
        assert!(out.effects().next().is_none());
        let err = model
            .workbench()
            .active_grid()
            .and_then(|g| g.error_label.clone());
        assert!(
            err.as_deref()
                .is_some_and(|e| e.contains("base table")),
            "{err:?}"
        );
    }
}
