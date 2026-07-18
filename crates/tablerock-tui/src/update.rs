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
            let token = model.mint_request_token();
            let context_revision = workbench.context_revision;
            workbench.catalog = CatalogModel::Loading {
                request_token: token,
                context_revision,
            };
            model.set_workbench(workbench);
            model.set_screen(Screen::Workbench);
            model.set_action(ActionId::Disconnect);
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
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                // First page from Execute/Browse stamps the result_token seed.
                if start_row == 0 {
                    grid.result_token = request_token;
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
            context_revision,
            rows_loaded,
            truncated,
            ..
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
        Message::Engine(EngineMsg::GridCancelDispatched { .. }) => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_cancel_requested();
            }
            Update::render()
        }
        Message::Engine(EngineMsg::GridCancelled { label, .. }) => {
            if let Some(grid) = model.workbench_mut().active_grid_mut() {
                grid.mark_cancelled();
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
        Message::Activate => Update::unchanged(),
        Message::RequestRedraw => Update::render(),
        Message::Quit => Update::with_effect(Effect::Exit),
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
            let token = model.mint_request_token();
            model.set_profiles(ProfileListState::Loading {
                request_token: token,
            });
            model.set_confirm(None);
            match confirm {
                ConfirmDialog::RemoveProfile { id_hex, .. } => Update {
                    render: true,
                    effect: Some(Effect::DeleteProfile {
                        request_token: token,
                        profile_id_hex: id_hex,
                    }),
                },
                ConfirmDialog::RemoveGroup { name } => Update {
                    render: true,
                    effect: Some(Effect::DeleteGroup {
                        request_token: token,
                        group_name: name,
                    }),
                },
                ConfirmDialog::CloseDirtyTab { index, .. } => {
                    model.workbench_mut().force_close_tab(index);
                    model.set_action(ActionId::Disconnect);
                    Update::render()
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
        | ActionId::Submit
        | ActionId::Cancel => Update::unchanged(),
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
        return Update {
            render: true,
            effect: Some(Effect::BrowseTable {
                request_token: token,
                session_id_hex,
                context_revision,
                schema,
                table,
            }),
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
            Message::Engine(EngineMsg::GridCancelDispatched { request_token: 1 }),
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
        let observed = update(
            &mut model,
            Message::Engine(EngineMsg::GridCancelled {
                request_token: 1,
                label: "server confirmed cancelled".into(),
            }),
        );
        assert!(observed.needs_render());
        let grid = model.workbench().active_grid().unwrap();
        assert_eq!(grid.operation, GridOperationState::Cancelled);
        assert_eq!(grid.operation.label(), "cancelled");
        assert_eq!(
            grid.error_label.as_deref(),
            Some("server confirmed cancelled")
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
            }),
        );
        assert!(ok.needs_render());
        let grid = model.workbench().active_grid().unwrap();
        assert_eq!(grid.row_count, 1);
        assert_eq!(grid.columns, ["id"]);
        assert_eq!(grid.operation, GridOperationState::Completed);
        assert!(grid.is_resident(0));
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
}
