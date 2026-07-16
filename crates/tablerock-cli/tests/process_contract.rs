use std::process::{Command, Stdio};

use tablerock_cli::root_message_channel;
use tablerock_tui::{Message, subscriptions::ENGINE_EVENT_CAPACITY};

#[test]
fn non_tty_execution_is_explicit_and_safe() {
    let output = Command::new(env!("CARGO_BIN_EXE_tablerock-cli"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run TableRock without a TTY");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"TableRock: interactive terminal required\n");
}

#[test]
fn post_mapping_root_port_uses_the_declared_hard_capacity() {
    let (sender, mut receiver) = root_message_channel();
    for _ in 0..ENGINE_EVENT_CAPACITY {
        sender
            .try_send_event(Message::RequestRedraw)
            .expect("message within declared capacity");
    }
    assert_eq!(
        sender.try_send_event(Message::RequestRedraw),
        Ok(tablerock_cli::SendOutcome::ResyncRequired)
    );
    assert_eq!(
        receiver.try_recv(),
        Ok(tablerock_cli::Delivery::ResyncRequired)
    );
    assert_eq!(
        receiver.try_recv(),
        Ok(tablerock_cli::Delivery::Event(Message::RequestRedraw))
    );
}
