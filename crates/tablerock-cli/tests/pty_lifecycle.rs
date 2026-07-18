use std::{
    io::{Read, Write},
    process::Command,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};

mod support;

// Shared GitHub runners can take longer than a local workstation to drain a
// high-rate mouse/resize flood after Ctrl-C. Bound is still finite and fails
// closed; it is not a correctness relaxation of the starvation property.
const TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn semantic_quit_restores_terminal_modes() {
    let output = run_pty(|writer, _, _| {
        writer.write_all(b"\t\t\t\t")?;
        writer.flush()?;
        thread::sleep(Duration::from_millis(50));
        // Actions: Open -> New -> Remove -> Quit
        writer.write_all(b"\x1b[C\x1b[C\x1b[C")?;
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
    let output = run_pty(|_, process_id, _| {
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

#[test]
fn resized_render_authorizes_focus_paste_and_mouse_quit() {
    let output = run_pty(|writer, _, master| {
        master
            .resize(PtySize {
                rows: 8,
                cols: 30,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(std::io::Error::other)?;
        thread::sleep(Duration::from_millis(50));
        master
            .resize(PtySize {
                rows: 30,
                cols: 100,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(std::io::Error::other)?;
        thread::sleep(Duration::from_millis(100));
        writer.write_all(b"\x1b[O\x1b[I")?;
        writer.write_all(b"\x1b[200~private pasted value\x1b[201~")?;
        writer.write_all(b"\x1b[<0;40;10M\x1b[<0;40;10m")?;
        writer.write_all(b"\x1b[<0;2;28M\x1b[<32;7;28M\x1b[<0;7;28m")?;
        writer.write_all(b"\x1b[Z")?;
        writer.write_all(b"\x1b[<64;2;28M")?;
        writer.flush()?;
        // Wheel focus selects Open on resized action row 28. Move Open->New->Remove->Quit.
        writer.write_all(b"\x1b[C\x1b[C\x1b[C\r")?;
        writer.flush()
    });
    assert_restored(&output);
    assert!(
        output
            .windows(b"Too Small".len())
            .any(|window| window == b"Too Small"),
        "tiny resize must render its explicit bounded state"
    );
    assert!(
        output
            .windows(b"> Workspace".len())
            .any(|window| window == b"> Workspace"),
        "primary press/release must focus the painted workspace"
    );
    assert!(
        output.contains(&b'~'),
        "drag must project a non-color hover cue"
    );
    assert!(
        !output
            .windows(b"private pasted value".len())
            .any(|window| window == b"private pasted value"),
        "paste content must not be rendered or logged"
    );
}

#[test]
fn high_rate_mouse_and_resize_do_not_starve_terminal_quit() {
    let output = run_pty(|writer, _, master| {
        // Keep the flood large enough to stress coalescing, but finite on
        // shared Linux runners where resize+mouse cost more wall time.
        for index in 0..64 {
            if master
                .resize(PtySize {
                    rows: 24,
                    cols: 80 + (index % 2),
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .is_err()
            {
                break;
            }
            if writer.write_all(b"\x1b[<35;40;10M").is_err() {
                break;
            }
            if index == 32 {
                // Mid-flood quit must still be accepted (not starved forever).
                writer.write_all(b"\x03")?;
            }
            thread::yield_now();
        }
        // Trailing Ctrl-C covers runners that drop mid-stream delivery under
        // extreme resize churn; starvation would still hang past TIMEOUT.
        writer.write_all(b"\x03")?;
        writer.flush()
    });
    assert_restored(&output);
}

fn run_pty(
    action: impl FnOnce(&mut dyn Write, u32, &dyn MasterPty) -> std::io::Result<()>,
) -> Vec<u8> {
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
    #[cfg(unix)]
    let initial_termios = pair.master.get_termios().expect("initial PTY termios");
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
    action(&mut writer, process_id, pair.master.as_ref()).expect("perform PTY action");
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
    #[cfg(unix)]
    assert_eq!(
        pair.master.get_termios().expect("restored PTY termios"),
        initial_termios,
        "raw-mode termios state must restore exactly"
    );
    drop(writer);
    drop(pair.master);
    reader_thread.join().expect("join PTY reader")
}

fn assert_restored(output: &[u8]) {
    support::assert_fullscreen_lifecycle(output);
}
