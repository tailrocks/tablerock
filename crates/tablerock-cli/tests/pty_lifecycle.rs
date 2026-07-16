use std::{
    io::{Read, Write},
    process::Command,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

const TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn semantic_quit_restores_terminal_modes() {
    let output = run_pty(|writer, _| {
        writer.write_all(b"\t\t\t\t")?;
        writer.flush()?;
        thread::sleep(Duration::from_millis(50));
        writer.write_all(b"\x1b[C")?;
        writer.flush()?;
        thread::sleep(Duration::from_millis(50));
        writer.write_all(b"\r")?;
        writer.flush()
    });
    assert_restored(&output);
}

#[cfg(unix)]
#[test]
fn terminate_signal_restores_terminal_modes() {
    let output = run_pty(|_, process_id| {
        let status = Command::new("kill")
            .args(["-TERM", &process_id.to_string()])
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other("kill command failed"))
        }
    });
    assert_restored(&output);
}

fn run_pty(action: impl FnOnce(&mut dyn Write, u32) -> std::io::Result<()>) -> Vec<u8> {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("open PTY");
    let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_tablerock-cli"));
    command.env("TERM", "xterm-256color");
    let mut child = pair.slave.spawn_command(command).expect("spawn TableRock");
    drop(pair.slave);
    let process_id = child.process_id().expect("child process ID");
    let mut reader = pair.master.try_clone_reader().expect("clone PTY reader");
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let reader_thread = thread::spawn(move || {
        let mut output = Vec::new();
        let mut ready = false;
        let mut buffer = [0_u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(length) => {
                    output.extend_from_slice(&buffer[..length]);
                    if !ready && output.windows(5).any(|window| window == b"Ready") {
                        ready = true;
                        let _ = ready_sender.send(());
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => {
                    if error.raw_os_error() == Some(5)
                        || matches!(
                            error.kind(),
                            std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::UnexpectedEof
                        )
                    {
                        break;
                    }
                    panic!("read PTY output: {error}");
                }
            }
        }
        output
    });
    let mut writer = pair.master.take_writer().expect("open PTY writer");

    ready_receiver
        .recv_timeout(TIMEOUT)
        .expect("wait for first rendered frame");
    thread::sleep(Duration::from_millis(20));
    action(&mut writer, process_id).expect("perform PTY action");
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll child") {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill timed-out child");
            panic!("TableRock PTY child exceeded {TIMEOUT:?}");
        }
        thread::sleep(Duration::from_millis(20));
    };
    assert!(status.success(), "TableRock exited with {status:?}");
    drop(writer);
    drop(pair.master);
    reader_thread.join().expect("join PTY reader")
}

fn assert_restored(output: &[u8]) {
    for sequence in [
        b"\x1b[?1049h".as_slice(),
        b"\x1b[?25l".as_slice(),
        b"\x1b[?25h".as_slice(),
        b"\x1b[?2004l".as_slice(),
        b"\x1b[?1000l".as_slice(),
        b"\x1b[?1049l".as_slice(),
    ] {
        assert!(
            output
                .windows(sequence.len())
                .any(|window| window == sequence),
            "missing terminal sequence {sequence:?}"
        );
    }
}
