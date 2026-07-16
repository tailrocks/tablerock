//! Pure full-frame shell rendering and render-authorized hit geometry.

use ratatui_core::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::Style,
    terminal::Frame,
};
use termrock::{
    interaction::HitRegion,
    runtime::View,
    widgets::{
        Action, ActionBar, ActionBarState, Hint, HintBar, Panel, PanelEmphasis, StatusBar,
        StatusBarState, StatusSlot, Tab, Tabs, TabsState,
    },
};

use crate::{ActionId, FocusRegion, LayoutMode, Model, Screen, ShellTarget};

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

impl View<Model> for ShellView {
    fn render(&self, model: &Model, frame: &mut Frame<'_>, area: Rect) {
        let _ = self.render_with_geometry(model, frame, area);
    }
}

impl ShellView {
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
            model.focus() == FocusRegion::Context,
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
    let label = if model.focus() == FocusRegion::Tabs {
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
        focused: model.focus() == FocusRegion::Tabs,
        regions: Vec::new(),
    };
    frame.render_stateful_widget(
        &Tabs {
            tabs: &tabs,
            gap: 1,
            theme: &model.theme,
        },
        area,
        &mut state,
    );
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
                model.focus() == FocusRegion::Catalog,
            );
            geometry.push(ShellTarget::Focus(FocusRegion::Catalog), columns[0]);
            render_panel(
                model,
                frame,
                columns[1],
                "Workspace",
                model.focus() == FocusRegion::Content,
            );
            geometry.push(ShellTarget::Focus(FocusRegion::Content), columns[1]);
        }
        LayoutMode::Narrow => {
            let (title, focused, target) = match model.focus() {
                FocusRegion::Catalog => ("Catalog", true, FocusRegion::Catalog),
                FocusRegion::Content => ("Workspace", true, FocusRegion::Content),
                _ => ("Connections", false, FocusRegion::Context),
            };
            render_panel(model, frame, area, title, focused);
            geometry.push(ShellTarget::Focus(target), area);
        }
        LayoutMode::TooSmall => {}
    }
}

fn render_actions(model: &Model, frame: &mut Frame<'_>, area: Rect, geometry: &mut ShellGeometry) {
    let open_label =
        if model.focus() == FocusRegion::Actions && model.selected_action() == ActionId::Open {
            "> Open"
        } else if model.hovered() == Some(ShellTarget::Action(ActionId::Open)) {
            "~ Open"
        } else {
            "Open"
        };
    let actions = [
        Action {
            id: ActionId::Open,
            label: open_label,
            enabled: true,
            style: None,
        },
        Action {
            id: ActionId::Quit,
            label: if model.focus() == FocusRegion::Actions
                && model.selected_action() == ActionId::Quit
            {
                "> Quit"
            } else if model.hovered() == Some(ShellTarget::Action(ActionId::Quit)) {
                "~ Quit"
            } else {
                "Quit"
            },
            enabled: true,
            style: None,
        },
    ];
    let mut state = ActionBarState {
        focused: (model.focus() == FocusRegion::Actions).then_some(model.selected_action()),
        regions: Vec::new(),
    };
    frame.render_stateful_widget(
        &ActionBar {
            actions: &actions,
            gap: " ",
            theme: &model.theme,
        },
        area,
        &mut state,
    );
    for region in state.regions {
        geometry.push(ShellTarget::Action(region.id), region.area);
    }
}

fn render_hints(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let actions = model.focus() == FocusRegion::Actions;
    let hints = if actions {
        [
            Hint {
                chord: "Enter",
                label: "Activate",
                priority: 0,
                visible: true,
            },
            Hint {
                chord: "Left/Right",
                label: "Choose action",
                priority: 1,
                visible: true,
            },
            Hint {
                chord: "Tab",
                label: "Next focus",
                priority: 2,
                visible: true,
            },
        ]
    } else {
        [
            Hint {
                chord: "Tab",
                label: "Next focus",
                priority: 0,
                visible: true,
            },
            Hint {
                chord: "Shift-Tab",
                label: "Previous focus",
                priority: 1,
                visible: true,
            },
            Hint {
                chord: "",
                label: "",
                priority: 2,
                visible: false,
            },
        ]
    };
    frame.render_widget(
        &HintBar {
            hints: &hints,
            separator: " • ",
            theme: &model.theme,
        },
        area,
    );
}

fn render_status(model: &Model, frame: &mut Frame<'_>, area: Rect, geometry: &mut ShellGeometry) {
    let focus = if model.focus() == FocusRegion::Footer {
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
        &StatusBar {
            left: &left,
            right: &right,
            theme: &model.theme,
            alpha: 1.0,
        },
        area,
        &mut state,
    );
    for region in state.regions {
        geometry.push(ShellTarget::Focus(FocusRegion::Footer), region.area);
    }
}

fn render_panel(model: &Model, frame: &mut Frame<'_>, area: Rect, title: &str, focused: bool) {
    let focused_title = focused.then(|| format!("> {title}"));
    let panel = Panel {
        title: Some(focused_title.as_deref().unwrap_or(title)),
        emphasis: if focused {
            PanelEmphasis::Focused
        } else {
            PanelEmphasis::Normal
        },
        style: None,
        theme: &model.theme,
    };
    frame.render_widget(&panel, area);
}
