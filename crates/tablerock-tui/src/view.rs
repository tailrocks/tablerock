//! Pure full-frame shell rendering and render-authorized hit geometry.

use ratatui_core::text::Line;
use ratatui_core::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::Style,
    terminal::Frame,
};
use termrock::{
    interaction::HitRegion,
    widgets::{
        Action, ActionBar, ActionBarState, Form, FormField, FormSection, FormState, Panel,
        PanelEmphasis, StatusBar, StatusBarState, StatusSlot, Tab, Tabs, TabsState, Tree, TreeNode,
        TreeNodeStatus, TreeState, render_hint_bar,
    },
};

use crate::{
    ActionId, FocusRegion, LayoutMode, Model, Screen, ShellKeyAction, ShellTarget,
    model::editor::EditorField,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct ShellView;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShellGeometry {
    regions: Vec<HitRegion<ShellTarget>>,
}

impl ShellGeometry {
    #[must_use]
    pub fn target_at(&self, column: u16, row: u16) -> Option<ShellTarget> {
        let position = Position::new(column, row);
        self.regions
            .iter()
            .rev()
            .find(|region| region.area.contains(position))
            .map(|region| region.id)
    }

    fn push(&mut self, id: ShellTarget, area: Rect) {
        if !area.is_empty() {
            self.regions.push(HitRegion { id, area });
        }
    }

    pub(crate) fn focus_area(&self, focus: FocusRegion) -> Option<Rect> {
        self.regions.iter().find_map(|region| {
            let owns_focus = match region.id {
                ShellTarget::Focus(candidate) => candidate == focus,
                ShellTarget::Action(_) => focus == FocusRegion::Actions,
            };
            owns_focus.then_some(region.area)
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellTab {
    Connections,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusId {
    State,
    Focus,
}

impl ShellView {
    pub fn render(&self, model: &Model, frame: &mut Frame<'_>, area: Rect) {
        let _ = self.render_with_geometry(model, frame, area);
    }

    #[must_use]
    pub fn render_with_geometry(
        &self,
        model: &Model,
        frame: &mut Frame<'_>,
        area: Rect,
    ) -> ShellGeometry {
        let mut geometry = ShellGeometry::default();
        if model.layout_mode() == LayoutMode::TooSmall {
            render_panel(model, frame, area, "TableRock — Too Small", true);
            return geometry;
        }

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Min(3),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);
        render_panel(
            model,
            frame,
            rows[0],
            "TableRock — Connections",
            model.focus() == Some(FocusRegion::Context),
        );
        geometry.push(ShellTarget::Focus(FocusRegion::Context), rows[0]);
        render_tabs(model, frame, rows[1], &mut geometry);
        render_body(model, frame, rows[2], &mut geometry);
        render_actions(model, frame, rows[3], &mut geometry);
        render_hints(model, frame, rows[4]);
        render_status(model, frame, rows[5], &mut geometry);
        if model.password_prompt().is_some() {
            render_password_prompt_overlay(model, frame, rows[2]);
        } else if model.confirm().is_some() {
            render_confirm_overlay(model, frame, rows[2]);
        }
        geometry
    }
}

fn render_confirm_overlay(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    use ratatui_core::widgets::Widget;
    let Some(confirm) = model.confirm() else {
        return;
    };
    let (title, body) = match confirm {
        crate::model::ConfirmDialog::RemoveProfile { name, id_hex } => (
            "Remove profile?",
            format!("Remove '{name}' ({id_hex})? Active sessions must be reviewed."),
        ),
        crate::model::ConfirmDialog::RemoveGroup { name } => (
            "Remove group?",
            format!("Remove group '{name}'? Members become ungrouped."),
        ),
    };
    let panel = Panel::new(&model.theme)
        .title(title)
        .emphasis(PanelEmphasis::Focused);
    frame.render_widget(&panel, area);
    if area.height > 2 && area.width > 2 {
        let inner = Rect {
            x: area.x.saturating_add(1),
            y: area.y.saturating_add(1),
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };
        Line::from(body).render(inner, frame.buffer_mut());
    }
}

fn render_password_prompt_overlay(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    use ratatui_core::widgets::Widget;
    let Some(prompt) = model.password_prompt() else {
        return;
    };
    let panel = Panel::new(&model.theme)
        .title("Password required")
        .emphasis(PanelEmphasis::Focused);
    frame.render_widget(&panel, area);
    if area.height > 2 && area.width > 2 {
        let inner = Rect {
            x: area.x.saturating_add(1),
            y: area.y.saturating_add(1),
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };
        let masked = "•".repeat(prompt.buffer.chars().count().min(64));
        Line::from(format!(
            "profile {} password: {masked}",
            prompt.profile_id_hex
        ))
        .render(inner, frame.buffer_mut());
    }
}

fn render_tabs(model: &Model, frame: &mut Frame<'_>, area: Rect, geometry: &mut ShellGeometry) {
    let label = if model.focus() == Some(FocusRegion::Tabs) {
        "> Connections"
    } else {
        "Connections"
    };
    let tabs = [Tab {
        id: ShellTab::Connections,
        label,
        glyph: None,
        active: true,
        enabled: true,
    }];
    let mut state = TabsState {
        selected: Some(ShellTab::Connections),
        hovered: None,
        focused: model.focus() == Some(FocusRegion::Tabs),
        regions: Vec::new(),
    };
    frame.render_stateful_widget(Tabs::new(&tabs, &model.theme).gap(1), area, &mut state);
    for region in state.regions {
        geometry.push(ShellTarget::Focus(FocusRegion::Tabs), region.area);
    }
}

fn render_body(model: &Model, frame: &mut Frame<'_>, area: Rect, geometry: &mut ShellGeometry) {
    if model.screen() == Screen::ConnectionPicker {
        render_panel(model, frame, area, "Connection Picker", true);
        geometry.push(ShellTarget::Focus(FocusRegion::Content), area);
        return;
    }
    match model.layout_mode() {
        LayoutMode::Wide | LayoutMode::Medium => {
            let catalog = if model.layout_mode() == LayoutMode::Wide {
                Constraint::Length(30)
            } else {
                Constraint::Percentage(32)
            };
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([catalog, Constraint::Min(20)])
                .split(area);
            render_panel(
                model,
                frame,
                columns[0],
                "Catalog",
                model.focus() == Some(FocusRegion::Catalog),
            );
            geometry.push(ShellTarget::Focus(FocusRegion::Catalog), columns[0]);
            render_panel(
                model,
                frame,
                columns[1],
                "Workspace",
                model.focus() == Some(FocusRegion::Content),
            );
            geometry.push(ShellTarget::Focus(FocusRegion::Content), columns[1]);
        }
        LayoutMode::Narrow => {
            let (title, focused, target) = match model.focus() {
                Some(FocusRegion::Catalog) => ("Catalog", true, FocusRegion::Catalog),
                Some(FocusRegion::Content) => ("Workspace", true, FocusRegion::Content),
                _ => ("Connections", false, FocusRegion::Context),
            };
            render_panel(model, frame, area, title, focused);
            geometry.push(ShellTarget::Focus(target), area);
        }
        LayoutMode::TooSmall => {}
    }
}

fn render_actions(model: &Model, frame: &mut Frame<'_>, area: Rect, geometry: &mut ShellGeometry) {
    let open = action_label(model, ActionId::Open, "Open");
    let new = action_label(model, ActionId::New, "New");
    let save = action_label(model, ActionId::Save, "Save");
    let test = action_label(model, ActionId::Test, "Test");
    let connect = action_label(model, ActionId::Connect, "Connect");
    let disconnect = action_label(model, ActionId::Disconnect, "Disconnect");
    let next_db = action_label(model, ActionId::NextDatabase, "Next DB");
    let submit = action_label(model, ActionId::Submit, "Submit");
    let cancel = action_label(model, ActionId::Cancel, "Cancel");
    let quit = action_label(model, ActionId::Quit, "Quit");
    let remove = action_label(model, ActionId::Remove, "Remove");
    let actions: Vec<Action<'_, ActionId>> =
        if model.password_prompt().is_some() || model.confirm().is_some() {
            vec![
                Action {
                    id: ActionId::Submit,
                    label: submit.as_str(),
                    enabled: true,
                    style: None,
                },
                Action {
                    id: ActionId::Cancel,
                    label: cancel.as_str(),
                    enabled: true,
                    style: None,
                },
                Action {
                    id: ActionId::Quit,
                    label: quit.as_str(),
                    enabled: true,
                    style: None,
                },
            ]
        } else {
            match model.screen() {
                crate::Screen::Editor => vec![
                    Action {
                        id: ActionId::Save,
                        label: save.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Test,
                        label: test.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Connect,
                        label: connect.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Cancel,
                        label: cancel.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Quit,
                        label: quit.as_str(),
                        enabled: true,
                        style: None,
                    },
                ],
                crate::Screen::Workbench => vec![
                    Action {
                        id: ActionId::NextDatabase,
                        label: next_db.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Disconnect,
                        label: disconnect.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Quit,
                        label: quit.as_str(),
                        enabled: true,
                        style: None,
                    },
                ],
                _ => vec![
                    Action {
                        id: ActionId::Open,
                        label: open.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::New,
                        label: new.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Remove,
                        label: remove.as_str(),
                        enabled: true,
                        style: None,
                    },
                    Action {
                        id: ActionId::Quit,
                        label: quit.as_str(),
                        enabled: true,
                        style: None,
                    },
                ],
            }
        };
    let mut state = ActionBarState {
        focused: (model.focus() == Some(FocusRegion::Actions)).then_some(model.selected_action()),
        regions: Vec::new(),
    };
    frame.render_stateful_widget(
        ActionBar::new(&actions, &model.theme).gap(" "),
        area,
        &mut state,
    );
    for region in state.regions {
        geometry.push(ShellTarget::Action(region.id), region.area);
    }
}

fn action_label(model: &Model, id: ActionId, base: &str) -> String {
    if model.focus() == Some(FocusRegion::Actions) && model.selected_action() == id {
        format!("> {base}")
    } else if model.hovered() == Some(ShellTarget::Action(id)) {
        format!("~ {base}")
    } else {
        base.to_owned()
    }
}

fn render_hints(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let mut keymap = model.keymap().clone();
    if model.focus() == Some(FocusRegion::Actions) {
        let _ = keymap.disable(ShellKeyAction::FocusPrevious);
        let _ = keymap.disable(ShellKeyAction::Quit);
    } else {
        let _ = keymap.disable(ShellKeyAction::Activate);
        let _ = keymap.disable(ShellKeyAction::ActionPrevious);
        let _ = keymap.disable(ShellKeyAction::ActionNext);
        let _ = keymap.disable(ShellKeyAction::Quit);
    }
    render_hint_bar(frame, area, &keymap.hint_spans(), &model.theme);
}

fn render_status(model: &Model, frame: &mut Frame<'_>, area: Rect, geometry: &mut ShellGeometry) {
    let focus = if model.focus() == Some(FocusRegion::Footer) {
        "[FOCUSED] Footer"
    } else {
        "Footer"
    };
    let left = [StatusSlot {
        id: StatusId::State,
        content: if model.engine_resync_required() {
            "Resync required"
        } else {
            "Ready"
        },
        priority: 0,
        min_width: 5,
        enabled: true,
        style: Style::new(),
        hover_style: None,
    }];
    let right = [StatusSlot {
        id: StatusId::Focus,
        content: focus,
        priority: 0,
        min_width: 6,
        enabled: true,
        style: Style::new(),
        hover_style: None,
    }];
    let mut state = StatusBarState::default();
    frame.render_stateful_widget(
        StatusBar::new(&left, &right, &model.theme).alpha(1.0),
        area,
        &mut state,
    );
    for region in state.regions {
        geometry.push(ShellTarget::Focus(FocusRegion::Footer), region.area);
    }
}

fn render_panel(model: &Model, frame: &mut Frame<'_>, area: Rect, title: &str, focused: bool) {
    let focused_title = focused.then(|| format!("> {title}"));
    let body = if title == "Workspace" || title.ends_with("Workspace") {
        if model.screen() == crate::Screen::Workbench {
            Some(model.workbench().context.line())
        } else {
            Some(model.profiles().status_line())
        }
    } else if title == "Catalog" && model.screen() == crate::Screen::Workbench {
        Some(model.workbench().catalog_status_line())
    } else {
        None
    };
    let panel = Panel::new(&model.theme)
        .title(focused_title.as_deref().unwrap_or(title))
        .emphasis(if focused {
            PanelEmphasis::Focused
        } else {
            PanelEmphasis::Normal
        });
    frame.render_widget(&panel, area);
    if area.height <= 2 || area.width <= 2 {
        return;
    }
    let inner = Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    let is_workspace = title == "Workspace" || title.ends_with("Workspace");
    let is_catalog = title == "Catalog" || title.ends_with("Catalog");
    if is_catalog && model.screen() == Screen::Workbench {
        render_workbench_catalog(model, frame, inner, body.as_deref());
        return;
    }
    if !is_workspace {
        if let Some(status) = body.as_ref() {
            use ratatui_core::widgets::Widget;
            Line::from(status.as_str()).render(inner, frame.buffer_mut());
        }
        return;
    }
    match model.screen() {
        Screen::Connections | Screen::ConnectionPicker => {
            render_connection_tree(model, frame, inner, body.as_deref());
        }
        Screen::Editor => render_connection_form(model, frame, inner),
        Screen::Workbench => render_workbench_facts(model, frame, inner, body.as_deref()),
    }
}

fn render_workbench_catalog(
    model: &Model,
    frame: &mut Frame<'_>,
    area: Rect,
    status: Option<&str>,
) {
    use ratatui_core::widgets::Widget;
    let mut content = area;
    if let Some(status) = status {
        Line::from(status).render(
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
            frame.buffer_mut(),
        );
        content.y = content.y.saturating_add(1);
        content.height = content.height.saturating_sub(1);
    }
    let filter = match &model.workbench().catalog {
        crate::model::catalog::CatalogModel::Loaded { filter, .. } => filter.clone(),
        _ => String::new(),
    };
    if !filter.is_empty() && content.height > 0 {
        Line::from(format!("filter: {filter}")).render(
            Rect {
                x: content.x,
                y: content.y,
                width: content.width,
                height: 1,
            },
            frame.buffer_mut(),
        );
        content.y = content.y.saturating_add(1);
        content.height = content.height.saturating_sub(1);
    }
    let visible = model.workbench().catalog.visible_nodes();
    if visible.is_empty() {
        return;
    }
    let labels: Vec<String> = visible.iter().map(|n| n.tree_label()).collect();
    let ids: Vec<String> = visible.iter().map(|n| n.id.clone()).collect();
    let depths: Vec<u16> = visible.iter().map(|n| n.depth).collect();
    let expanded: Vec<bool> = visible.iter().map(|n| n.expanded).collect();
    let branches: Vec<bool> = visible.iter().map(|n| n.branch).collect();
    let selected_key = match &model.workbench().catalog {
        crate::model::catalog::CatalogModel::Loaded { selected_id, .. } => selected_id.clone(),
        _ => None,
    };
    let tree_nodes: Vec<TreeNode<'_, String>> = ids
        .iter()
        .zip(labels.iter())
        .zip(depths.iter())
        .zip(expanded.iter())
        .zip(branches.iter())
        .map(|((((id, label), depth), exp), branch)| TreeNode {
            id: id.clone(),
            label: Line::from(label.as_str()),
            trailing: None,
            depth: *depth,
            branch: *branch,
            expanded: *exp,
            enabled: true,
            status: TreeNodeStatus::Ready,
        })
        .collect();
    let mut state = TreeState::new(selected_key);
    state.set_focused(model.focus() == Some(FocusRegion::Catalog));
    frame.render_stateful_widget(&Tree::new(&tree_nodes, &model.theme), content, &mut state);
}

fn render_connection_tree(model: &Model, frame: &mut Frame<'_>, area: Rect, status: Option<&str>) {
    use ratatui_core::widgets::Widget;
    let mut content = area;
    if let Some(status) = status {
        Line::from(status).render(
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
            frame.buffer_mut(),
        );
        content.y = content.y.saturating_add(1);
        content.height = content.height.saturating_sub(1);
    }
    let search = match model.profiles() {
        crate::model::profiles::ProfileListState::Loaded { search, .. } => search.clone(),
        _ => String::new(),
    };
    if !search.is_empty() && content.height > 0 {
        Line::from(format!("filter: {search}")).render(
            Rect {
                x: content.x,
                y: content.y,
                width: content.width,
                height: 1,
            },
            frame.buffer_mut(),
        );
        content.y = content.y.saturating_add(1);
        content.height = content.height.saturating_sub(1);
    }
    let (nodes, labels, depths, selected_key) = build_connection_tree_nodes(model);
    let tree_nodes: Vec<TreeNode<'_, String>> = nodes
        .iter()
        .zip(labels.iter())
        .zip(depths.iter())
        .map(|((id, label), depth)| {
            let is_group = id.starts_with("g:");
            TreeNode {
                id: id.clone(),
                label: Line::from(label.as_str()),
                trailing: None,
                depth: *depth,
                branch: is_group,
                expanded: is_group
                    && !model
                        .profiles()
                        .is_group_collapsed(id.strip_prefix("g:").unwrap_or("")),
                enabled: true,
                status: TreeNodeStatus::Ready,
            }
        })
        .collect();
    let mut state = TreeState::new(selected_key);
    state.set_focused(model.focus() == Some(FocusRegion::Content));
    frame.render_stateful_widget(&Tree::new(&tree_nodes, &model.theme), content, &mut state);
}

/// Build TermRock Tree projection: group branches + profile leaves.
fn build_connection_tree_nodes(
    model: &Model,
) -> (Vec<String>, Vec<String>, Vec<u16>, Option<String>) {
    let rows = model.profiles().visible_rows();
    let mut groups: Vec<Option<String>> = Vec::new();
    for row in &rows {
        let key = row.group.clone();
        if !groups.iter().any(|existing| existing == &key) {
            groups.push(key);
        }
    }
    groups.sort_by(|left, right| match (left, right) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(a), Some(b)) => a.cmp(b),
    });
    let mut ids = Vec::new();
    let mut labels = Vec::new();
    let mut depths = Vec::new();
    for group in groups {
        match &group {
            Some(name) => {
                ids.push(format!("g:{name}"));
                labels.push(name.clone());
                depths.push(0);
                if model.profiles().is_group_collapsed(name) {
                    continue;
                }
                for row in &rows {
                    if row.group.as_deref() == Some(name.as_str()) {
                        ids.push(format!("p:{}", row.id_hex));
                        labels.push(row.list_line());
                        depths.push(1);
                    }
                }
            }
            None => {
                for row in &rows {
                    if row.group.is_none() {
                        ids.push(format!("p:{}", row.id_hex));
                        labels.push(row.list_line());
                        depths.push(0);
                    }
                }
            }
        }
    }
    let selected_key = model
        .profiles()
        .selected_row()
        .map(|row| format!("p:{}", row.id_hex));
    (ids, labels, depths, selected_key)
}

fn render_connection_form(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    use ratatui_core::widgets::Widget;
    let editor = model.editor();
    let mut form_area = area;
    if let Some(error) = editor.validation_error.as_ref() {
        Line::from(format!("! {error}")).render(
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
            frame.buffer_mut(),
        );
        form_area.y = form_area.y.saturating_add(1);
        form_area.height = form_area.height.saturating_sub(1);
    } else if let Some(status) = editor.test_status.as_ref() {
        Line::from(format!("test: {status}")).render(
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
            frame.buffer_mut(),
        );
        form_area.y = form_area.y.saturating_add(1);
        form_area.height = form_area.height.saturating_sub(1);
    }
    // Owned display values live for this render frame.
    let engine = editor.field_value(EditorField::Engine);
    let name = editor.field_value(EditorField::Name);
    let group = editor.field_value(EditorField::Group);
    let environment = editor.field_value(EditorField::Environment);
    let host = editor.field_value(EditorField::Host);
    let port = editor.field_value(EditorField::Port);
    let database = editor.field_value(EditorField::Database);
    let username = editor.field_value(EditorField::Username);
    let password = editor.field_value(EditorField::Password);
    let password_source = editor.field_value(EditorField::PasswordSource);
    let tls_mode = editor.field_value(EditorField::TlsMode);

    let general = [
        FormField::new(
            EditorField::Engine,
            Line::from("Engine"),
            Line::from(engine.as_str()),
        ),
        FormField::new(
            EditorField::Name,
            Line::from("Name"),
            Line::from(name.as_str()),
        )
        .required(true),
        FormField::new(
            EditorField::Group,
            Line::from("Group"),
            Line::from(group.as_str()),
        ),
        FormField::new(
            EditorField::Environment,
            Line::from("Environment"),
            Line::from(environment.as_str()),
        ),
    ];
    let connection = [
        FormField::new(
            EditorField::Host,
            Line::from("Host"),
            Line::from(host.as_str()),
        )
        .required(true),
        FormField::new(
            EditorField::Port,
            Line::from("Port"),
            Line::from(port.as_str()),
        )
        .required(true),
        FormField::new(
            EditorField::Database,
            Line::from("Database"),
            Line::from(database.as_str()),
        ),
    ];
    let credentials = [
        FormField::new(
            EditorField::Username,
            Line::from("Username"),
            Line::from(username.as_str()),
        ),
        FormField::new(
            EditorField::Password,
            Line::from("Password"),
            Line::from(password.as_str()),
        ),
        FormField::new(
            EditorField::PasswordSource,
            Line::from("Password source"),
            Line::from(password_source.as_str()),
        ),
    ];
    let tls = [FormField::new(
        EditorField::TlsMode,
        Line::from("TLS mode"),
        Line::from(tls_mode.as_str()),
    )];
    let sections = [
        FormSection {
            title: Line::from("General"),
            fields: &general,
        },
        FormSection {
            title: Line::from("Connection"),
            fields: &connection,
        },
        FormSection {
            title: Line::from("Credentials"),
            fields: &credentials,
        },
        FormSection {
            title: Line::from("TLS"),
            fields: &tls,
        },
    ];
    let mut state = FormState::new(Some(editor.focused));
    state.set_active(model.focus() == Some(FocusRegion::Content));
    frame.render_stateful_widget(&Form::new(&sections, &model.theme), form_area, &mut state);
}

fn render_workbench_facts(model: &Model, frame: &mut Frame<'_>, area: Rect, _status: Option<&str>) {
    use ratatui_core::widgets::Widget;
    let wb = model.workbench();
    let mut y = area.y;
    let max_y = area.y.saturating_add(area.height);
    let lines = [
        wb.context.line(),
        wb.catalog_status_line(),
        wb.tabs
            .get(wb.selected_tab)
            .map(|tab| {
                format!(
                    "tab: {}{}{}",
                    tab.title,
                    if tab.dirty { " *" } else { "" },
                    if tab.running { " …" } else { "" }
                )
            })
            .unwrap_or_else(|| "tab: —".into()),
        wb.status.summary(),
        model
            .session()
            .map(|session| format!("session: {}", session.session_id_hex))
            .unwrap_or_default(),
    ];
    for line in lines {
        if y >= max_y || line.is_empty() {
            continue;
        }
        let clipped: String = line.chars().take(area.width as usize).collect();
        Line::from(clipped).render(
            Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            },
            frame.buffer_mut(),
        );
        y = y.saturating_add(1);
    }
}
