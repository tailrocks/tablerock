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
                    | ConfirmDialog::RenameTable { confirm_buffer, .. }
                    | ConfirmDialog::CancelBackend { confirm_buffer, .. }
                    | ConfirmDialog::TerminateBackend { confirm_buffer, .. } => {
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
        Message::Engine(EngineMsg::HealthOk { .. } | EngineMsg::HealthFailed { .. }) => {
            Update::unchanged()
        }
        Message::Engine(EngineMsg::TestOk {
            identity,
            elapsed_millis,
            ..
        }) => {
            model.editor_mut().test_status = Some(format!("ok: {identity} ({elapsed_millis} ms)"));
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
            ..
        }) => {
            model.set_session(Some(SessionFacts {
                session_id_hex: session_id_hex.clone(),
                identity: identity.clone(),
                temporary,
                engine_label: engine_label.clone(),
                status: Some("connected".into()),
            }));
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
            model.set_action(ActionId::Disconnect);
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
        }) => {
            if model.workbench().context_revision != context_revision {
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
                }
                if let Some(identity) = identity_columns {
                    grid.identity_columns = identity;
                    // Browse sets base_schema/table before the first page arrives.
                    // Ad-hoc SQL leaves base unset → stays read-only.
                    if grid.base_schema.is_some() && grid.base_table.is_some() {
                        grid.recompute_editability(safety, false);
                    }
                }
                grid.replace_page(
                    start_row, columns, cells, row_count, totals, bytes, truncated,
                );
                if complete {
                    grid.mark_completed();
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
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.rows_loaded = grid.rows_loaded.max(rows_loaded);
                if truncated {
                    grid.truncated = true;
                }
                grid.mark_completed();
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
            // After intent restore, load catalog for the restored context.
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
        Message::Engine(EngineMsg::SessionIntentFailed { .. }) => Update::unchanged(),
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
                if let Some(grid) = model.workbench_mut().active_grid_mut() {
                    grid.drafts.discard_all();
                    grid.cell_edit = None;
                    grid.mark_completed();
                    grid.error_label = Some(format!("applied {change_count}: {detail}"));
                }
                model.workbench_mut().mutation_review = None;
                model.workbench_mut().mark_active_dirty(false);
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
            foreign_column,
            filter_value,
            ..
        }) => {
            if model.workbench().context_revision != context_revision {
                return Update::unchanged();
            }
            let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
                return Update::unchanged();
            };
            let title = format!("{foreign_table} · {foreign_column}={filter_value}");
            model.workbench_mut().open_preview_tab(&title);
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.base_schema = Some(foreign_schema.clone());
                grid.base_table = Some(foreign_table.clone());
                grid.clear_server_controls();
                grid.add_filter_chip(foreign_column, "eq", Some(filter_value));
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
            model.workbench_mut().inspector = crate::model::inspector::InspectorModel {
                open: true,
                title: format!("{schema}.{table} structure"),
                kind_label: "structure".into(),
                text: columns.join("\n"),
                hex: String::new(),
                byte_len: columns.iter().map(|c| c.len() as u64).sum(),
                original_byte_len: None,
                stale: false,
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
        Message::Engine(EngineMsg::GridFailed {
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
        Message::Engine(EngineMsg::GridCancelDispatched { dispatch, .. }) => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_cancel_dispatch(&dispatch);
            }
            Update::render()
        }
        Message::Engine(EngineMsg::GridCancelled { label, .. }) => {
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
            attempt,
            next_delay_ms,
            ..
        }) => {
            if let Some(mut session) = model.session().cloned() {
                session.status = Some(format!(
                    "reconnecting attempt {attempt} (next {next_delay_ms} ms)"
                ));
                model.set_session(Some(session));
            }
            Update::render()
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
            Update {
                render: true,
                effect: Some(Effect::ConnectSession {
                    request_token: token,
                    draft: connection_draft_from_editor(model.editor()),
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
        ActionId::RunSql if model.screen() == Screen::Workbench => {
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
                return Update::unchanged();
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
                effect: Some(Effect::ExecuteSql {
                    request_token: token,
                    session_id_hex,
                    context_revision,
                    statement,
                }),
            }
        }
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
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                if !grid.drafts.is_empty() || grid.cell_edit.is_some() {
                    grid.drafts.discard_all();
                    grid.cell_edit = None;
                    model.workbench_mut().mutation_review = None;
                    model.workbench_mut().mark_active_dirty(false);
                    return Update::render();
                }
            }
            Update::unchanged()
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
        | ActionId::NextDatabase
        | ActionId::NextTab
        | ActionId::CloseTab
        | ActionId::PinTab
        | ActionId::NewSql
        | ActionId::RunSql
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
        | ActionId::CycleSort
        | ActionId::AddFilter
        | ActionId::ClearFilters
        | ActionId::ToggleColumn
        | ActionId::ResetColumns
        | ActionId::SaveColumns
        | ActionId::UndoStaged
        | ActionId::DiscardStaged
        | ActionId::ReviewMutations
        | ActionId::EditCell
        | ActionId::DeleteRow
        | ActionId::ApplyMutations
        | ActionId::FollowForeignKey
        | ActionId::ShowStructure
        | ActionId::TruncateTable
        | ActionId::DropTable
        | ActionId::RenameTable
        | ActionId::ShowActivity
        | ActionId::CancelBackend
        | ActionId::TerminateBackend
        | ActionId::Submit
        | ActionId::Cancel => Update::unchanged(),
    }
}

fn follow_foreign_key(model: &mut Model) -> Update {
    let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
        return Update::unchanged();
    };
    let context_revision = model.workbench().context_revision;
    let (schema, table, local_column, cell_value) = {
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
        let cell_value = grid.cell_at(grid.cursor_row, col_idx).text;
        (schema, table, local_column, cell_value)
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
            cell_value,
        }),
    }
}

fn show_structure(model: &mut Model) -> Update {
    let Some(session_id_hex) = model.session().map(|s| s.session_id_hex.clone()) else {
        return Update::unchanged();
    };
    let context_revision = model.workbench().context_revision;
    let (schema, table) = {
        let Some(grid) = model.workbench().active_grid() else {
            return Update::unchanged();
        };
        match (grid.base_schema.clone(), grid.base_table.clone()) {
            (Some(s), Some(t)) => (s, t),
            _ => return Update::unchanged(),
        }
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

fn collect_mutation_specs(
    model: &Model,
) -> Option<(String, String, String, Vec<crate::effect::MutationChangeSpec>)> {
    use crate::effect::MutationChangeSpec;
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
                ActionId::Cancel,
                ActionId::Quit,
            ],
            Screen::Workbench => &[
                ActionId::NextDatabase,
                ActionId::NextTab,
                ActionId::PinTab,
                ActionId::NewSql,
                ActionId::RunSql,
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
                ActionId::CopyMarkdown,
                ActionId::CopySqlInsert,
                ActionId::CopySqlUpdate,
                ActionId::CycleSort,
                ActionId::AddFilter,
                ActionId::ClearFilters,
                ActionId::ToggleColumn,
                ActionId::ResetColumns,
                ActionId::SaveColumns,
                ActionId::UndoStaged,
                ActionId::DiscardStaged,
                ActionId::ReviewMutations,
                ActionId::EditCell,
                ActionId::DeleteRow,
                ActionId::ApplyMutations,
                ActionId::FollowForeignKey,
                ActionId::ShowStructure,
                ActionId::TruncateTable,
                ActionId::DropTable,
                ActionId::RenameTable,
                ActionId::ShowActivity,
                ActionId::CancelBackend,
                ActionId::TerminateBackend,
                ActionId::CancelQuery,
                ActionId::Inspect,
                ActionId::CloseTab,
                ActionId::Disconnect,
                ActionId::Quit,
            ],
            Screen::Connections | Screen::ConnectionPicker => &[
                ActionId::Open,
                ActionId::New,
                ActionId::Remove,
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
            PasswordSourceChoice::DangerousPlaintext => PasswordSourceSpec::DangerousPlaintext,
        },
        tls_mode: match editor.tls_mode {
            TlsModeChoice::Off => TlsModeSpec::Off,
            TlsModeChoice::VerifyCa => TlsModeSpec::VerifyCa,
            TlsModeChoice::VerifyFull => TlsModeSpec::VerifyFull,
        },
        plaintext_acknowledged: editor.plaintext_acknowledged,
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
            }),
        );
        assert!(ok.needs_render());
        assert_eq!(
            model.editor().test_status.as_deref(),
            Some("ok: PostgreSQL 17 (12 ms)")
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
            }),
        );
        assert!(done.needs_render());
        let grid = model.workbench().active_grid().unwrap();
        assert_eq!(grid.operation, GridOperationState::Completed);
        assert_eq!(grid.rows_loaded, 2500);
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
}
