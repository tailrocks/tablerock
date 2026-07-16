use ratatui_core::{layout::Rect, terminal::Frame};
use termrock::{
    Theme,
    runtime::{UpdateResult, View},
    widgets::{Panel, PanelEmphasis},
};

#[derive(Debug, Default)]
pub struct Model {
    theme: Theme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    RequestRedraw,
}

pub fn update(_model: &mut Model, message: Message) -> UpdateResult {
    match message {
        Message::RequestRedraw => UpdateResult::redraw(),
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ShellView;

impl View<Model> for ShellView {
    fn render(&self, model: &Model, frame: &mut Frame<'_>, area: Rect) {
        let panel = Panel {
            title: Some("TableRock"),
            emphasis: PanelEmphasis::Focused,
            style: None,
            theme: &model.theme,
        };
        frame.render_widget(&panel, area);
    }
}
