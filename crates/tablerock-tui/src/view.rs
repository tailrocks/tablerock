//! Pure full-frame shell rendering.

use ratatui_core::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    terminal::Frame,
};
use termrock::{
    runtime::View,
    widgets::{
        Action, ActionBar, ActionBarState, Hint, HintBar, Panel, PanelEmphasis, StatusBar,
        StatusSlot, Tab, Tabs, TabsState,
    },
};

use crate::{ActionId, FocusRegion, LayoutMode, Model, Screen};

#[derive(Debug, Clone, Copy, Default)]
pub struct ShellView;

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
        if model.layout_mode() == LayoutMode::TooSmall {
            render_panel(model, frame, area, "TableRock — Too Small", true);
            return;
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
        render_tabs(model, frame, rows[1]);
        render_body(model, frame, rows[2]);
        render_actions(model, frame, rows[3]);
        render_hints(model, frame, rows[4]);
        render_status(model, frame, rows[5]);
    }
}

fn render_tabs(model: &Model, frame: &mut Frame<'_>, area: Rect) {
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
        },
        area,
        &mut state,
    );
}

fn render_body(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    if model.screen() == Screen::ConnectionPicker {
        render_panel(model, frame, area, "Connection Picker", true);
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
            render_panel(
                model,
                frame,
                columns[1],
                "Workspace",
                model.focus() == FocusRegion::Content,
            );
        }
        LayoutMode::Narrow => {
            let (title, focused) = match model.focus() {
                FocusRegion::Catalog => ("Catalog", true),
                FocusRegion::Content => ("Workspace", true),
                _ => ("Connections", false),
            };
            render_panel(model, frame, area, title, focused);
        }
        LayoutMode::TooSmall => {}
    }
}

fn render_actions(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let open_label =
        if model.focus() == FocusRegion::Actions && model.selected_action() == ActionId::Open {
            "> Open"
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
        },
        area,
        &mut state,
    );
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
        },
        area,
    );
}

fn render_status(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let focus = if model.focus() == FocusRegion::Footer {
        "[FOCUSED] Footer"
    } else {
        "Footer"
    };
    let left = [StatusSlot {
        id: StatusId::State,
        content: "Ready",
        priority: 0,
        min_width: 5,
        enabled: true,
        style: Style::new(),
    }];
    let right = [StatusSlot {
        id: StatusId::Focus,
        content: focus,
        priority: 0,
        min_width: 6,
        enabled: true,
        style: Style::new(),
    }];
    frame.render_widget(
        &StatusBar {
            left: &left,
            right: &right,
        },
        area,
    );
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
