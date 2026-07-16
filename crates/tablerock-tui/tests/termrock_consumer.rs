use ratatui_core::{backend::TestBackend, layout::Rect, terminal::Terminal};
use tablerock_tui::{Message, Model, ShellView, update};
use termrock::runtime::{Dirty, View, drive_frame};

#[test]
fn minimal_consumer_uses_termrock_runtime_and_render_contracts() {
    let mut model = Model::default();
    let update_result = update(&mut model, Message::RequestRedraw);

    assert_eq!(update_result.dirty(), Dirty::Redraw);
    assert!(update_result.effects().is_empty());

    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let view = ShellView;
    drive_frame(&mut terminal, &view, &model, Rect::new(0, 0, 24, 5), |_| {})
        .expect("render frame");

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(
        rendered.contains("TableRock"),
        "shell title should be rendered through TermRock"
    );
}

fn _view_contract_is_public(view: &impl View<Model>) {
    let _ = view;
}
