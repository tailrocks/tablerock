//! Pure full-frame shell rendering and render-authorized hit geometry.

use ratatui_core::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::Style,
    terminal::Frame,
};
use termrock::{
    interaction::HitRegion,
    widgets::{
        Action, ActionBar, ActionBarState, Panel, PanelEmphasis, StatusBar, StatusBarState,
        StatusSlot, Tab, Tabs, TabsState, render_hint_bar,
    },
};

use crate::{ActionId, FocusRegion, LayoutMode, Model, Screen, ShellKeyAction, ShellTarget};

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
        geometry
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
    let cancel = action_label(model, ActionId::Cancel, "Cancel");
    let quit = action_label(model, ActionId::Quit, "Quit");
    let actions: Vec<Action<'_, ActionId>> = match model.screen() {
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
                id: ActionId::Quit,
                label: quit.as_str(),
                enabled: true,
                style: None,
            },
        ],
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
        Some(model.profiles().status_line())
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
    // Prefer body when TermRock Panel supports content; fall back to title-only.
    let _ = body.as_ref();
    frame.render_widget(&panel, area);
    if area.height > 2 && area.width > 2 {
        use ratatui_core::widgets::Widget;
        let mut y = area.y.saturating_add(1);
        let max_y = area.y.saturating_add(area.height.saturating_sub(1));
        let x = area.x.saturating_add(1);
        let width = area.width.saturating_sub(2);
        if let Some(status) = body.as_ref() {
            let text = ratatui_core::text::Line::from(status.as_str());
            text.render(
                Rect {
                    x,
                    y,
                    width,
                    height: 1,
                },
                frame.buffer_mut(),
            );
            y = y.saturating_add(1);
        }
        if (title == "Workspace" || title.ends_with("Workspace"))
            && matches!(
                model.screen(),
                crate::Screen::Connections | crate::Screen::ConnectionPicker
            )
        {
            if let crate::model::profiles::ProfileListState::Loaded { rows, .. } = model.profiles()
            {
                for row in rows {
                    if y >= max_y {
                        break;
                    }
                    let line = row.list_line();
                    let clipped = if line.len() > width as usize {
                        line.chars().take(width as usize).collect::<String>()
                    } else {
                        line
                    };
                    ratatui_core::text::Line::from(clipped).render(
                        Rect {
                            x,
                            y,
                            width,
                            height: 1,
                        },
                        frame.buffer_mut(),
                    );
                    y = y.saturating_add(1);
                }
            }
        }
        if model.screen() == crate::Screen::Editor
            && (title == "Workspace" || title.ends_with("Workspace"))
        {
            use crate::model::editor::EditorField;
            for field in [
                EditorField::Engine,
                EditorField::Name,
                EditorField::Group,
                EditorField::Environment,
                EditorField::Host,
                EditorField::Port,
                EditorField::Database,
                EditorField::Username,
                EditorField::Password,
                EditorField::PasswordSource,
                EditorField::TlsMode,
            ] {
                if y >= max_y {
                    break;
                }
                let marker = if model.editor().focused == field {
                    ">"
                } else {
                    " "
                };
                let line = format!("{marker} {field:?}: {}", model.editor().field_value(field));
                let clipped: String = line.chars().take(width as usize).collect();
                ratatui_core::text::Line::from(clipped).render(
                    Rect {
                        x,
                        y,
                        width,
                        height: 1,
                    },
                    frame.buffer_mut(),
                );
                y = y.saturating_add(1);
            }
            if let Some(error) = &model.editor().validation_error
                && y < max_y
            {
                let clipped: String = format!("! {error}").chars().take(width as usize).collect();
                ratatui_core::text::Line::from(clipped).render(
                    Rect {
                        x,
                        y,
                        width,
                        height: 1,
                    },
                    frame.buffer_mut(),
                );
                y = y.saturating_add(1);
            }
            if let Some(status) = &model.editor().test_status
                && y < max_y
            {
                let clipped: String = format!("test: {status}")
                    .chars()
                    .take(width as usize)
                    .collect();
                ratatui_core::text::Line::from(clipped).render(
                    Rect {
                        x,
                        y,
                        width,
                        height: 1,
                    },
                    frame.buffer_mut(),
                );
            }
        }
    }
}
