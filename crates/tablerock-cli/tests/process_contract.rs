use std::process::{Command, Stdio};

use tablerock_cli::root_message_channel;
use tablerock_tui::{Message, subscriptions::ENGINE_EVENT_CAPACITY};

#[test]
fn non_tty_execution_is_explicit_and_safe() {
    let output = Command::new(env!("CARGO_BIN_EXE_tablerock"))
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
fn version_exits_before_terminal_initialization() {
    for argument in ["--version", "-V"] {
        let output = Command::new(env!("CARGO_BIN_EXE_tablerock"))
            .arg(argument)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("print TableRock version without a TTY");

        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).expect("UTF-8 version output"),
            format!("tablerock {}\n", env!("CARGO_PKG_VERSION"))
        );
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn support_bundle_exits_before_terminal_and_emits_only_safe_schema() {
    let output = Command::new(env!("CARGO_BIN_EXE_tablerock"))
        .arg("--support-bundle")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env(
            "DATABASE_URL",
            "postgres://admin:secret@private-host/database",
        )
        .output()
        .expect("print support bundle without a TTY");

    assert!(output.status.success());
    let bundle = String::from_utf8(output.stdout).expect("UTF-8 support bundle");
    assert!(bundle.starts_with("schema=1\nclient.version="));
    assert!(bundle.contains("diagnostics.count=0\n"));
    for forbidden in ["admin", "secret", "private-host", "database"] {
        assert!(!bundle.contains(forbidden));
    }
    assert!(output.stderr.is_empty());
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
