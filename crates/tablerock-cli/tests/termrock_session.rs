use tablerock_cli::{EventStream, Session, SessionOptions};

#[test]
fn cli_owns_scoped_session_contract_without_live_terminal_side_effects() {
    let options = SessionOptions {
        alternate_screen: false,
        mouse_capture: false,
        bracketed_paste: false,
        raw_mode: false,
        ..SessionOptions::default()
    };
    let mut session = Session::enter(Vec::new(), options).expect("in-memory session");

    session.restore().expect("restore in-memory session");
    session.restore().expect("restore remains idempotent");
}

fn _event_stream_contract_is_public() -> EventStream {
    EventStream::new()
}
