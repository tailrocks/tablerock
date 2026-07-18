use std::{
    error::Error,
    fmt,
    io::{self, IsTerminal},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Mutex, MutexGuard},
};

use futures_util::StreamExt;
use ratatui_core::terminal::Terminal;
use ratatui_crossterm::CrosstermBackend;
use tablerock_tui::subscriptions::{Subscription, root_subscriptions};
use tablerock_tui::{Effect, Message, Model, ShellView, update};
use termrock::crossterm::{Session, SessionOptions};

use crate::{
    Delivery, EventStream, IngressReceiver, IngressSender, InputAdapter, bounded_ingress,
    effects::EffectExecutor,
};

/// Bounded post-mapping ingress for root subscription messages.
pub type RootMessageSender = IngressSender<Message, RootProgress>;
pub type RootMessageReceiver = IngressReceiver<Message, RootProgress>;

/// The shipped Phase 1 shell has no progress payload; Phase 2 supplies its mapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootProgress {
    #[cfg(test)]
    Redraw,
}

enum LoopInput {
    Signal,
    Root(Option<Delivery<Message, RootProgress>>),
    Terminal(Option<io::Result<crossterm::event::Event>>),
    /// BoundedAutomatic continuous health probe interval.
    HealthTick,
}

#[derive(Debug)]
pub enum RunError {
    NonInteractive,
    Runtime(io::Error),
    Terminal(io::Error),
    Input(io::Error),
    Signal(io::Error),
    PrimaryAndRestore {
        primary: Box<Self>,
        restore: io::Error,
    },
    Panicked,
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonInteractive => formatter.write_str("interactive terminal required"),
            Self::Runtime(_) => formatter.write_str("failed to start the async runtime"),
            Self::Terminal(_) => formatter.write_str("terminal operation failed"),
            Self::Input(_) => formatter.write_str("terminal input failed"),
            Self::Signal(_) => formatter.write_str("signal handler failed"),
            Self::PrimaryAndRestore { .. } => {
                formatter.write_str("terminal restoration failed after another error")
            }
            Self::Panicked => formatter.write_str("TableRock stopped after an internal panic"),
        }
    }
}

impl Error for RunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Runtime(error)
            | Self::Terminal(error)
            | Self::Input(error)
            | Self::Signal(error) => Some(error),
            Self::PrimaryAndRestore { restore, .. } => Some(restore),
            Self::NonInteractive | Self::Panicked => None,
        }
    }
}

impl RunError {
    #[must_use]
    pub fn primary(&self) -> Option<&Self> {
        match self {
            Self::PrimaryAndRestore { primary, .. } => Some(primary),
            _ => None,
        }
    }
}

#[must_use]
pub fn root_message_channel() -> (RootMessageSender, RootMessageReceiver) {
    let capacity = root_subscriptions()
        .into_iter()
        .find_map(|subscription| match subscription {
            Subscription::EngineEvents { capacity } => Some(capacity),
            Subscription::TerminalInput | Subscription::Signals => None,
        })
        .expect("root subscriptions declare engine events");
    bounded_ingress(capacity)
}

/// Run TableRock and contain panics after terminal restoration.
pub fn run_caught() -> Result<(), RunError> {
    run_caught_boundary(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(RunError::Runtime)?
            .block_on(run())
    })
}

fn run_caught_boundary(operation: impl FnOnce() -> Result<(), RunError>) -> Result<(), RunError> {
    static PANIC_HOOK: Mutex<()> = Mutex::new(());
    let lock = PANIC_HOOK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _hook = PanicHookGuard::suppress(lock);
    let outcome = catch_unwind(AssertUnwindSafe(operation));
    outcome.unwrap_or(Err(RunError::Panicked))
}

/// Own the sole live terminal session and root event loop.
pub async fn run() -> Result<(), RunError> {
    let (ingress, root_messages) = root_message_channel();
    let executor = EffectExecutor::open_default(ingress)
        .map_err(|error| RunError::Runtime(io::Error::other(error)))?;
    run_with_root_messages_and_executor(root_messages, executor).await
}

/// Run with an injected bounded stream after source-specific semantic mapping.
pub async fn run_with_root_messages(root_messages: RootMessageReceiver) -> Result<(), RunError> {
    let (ingress, _drop_receiver) = root_message_channel();
    let path = std::env::temp_dir().join(format!("tablerock-cli-run-{}.db", std::process::id()));
    let actor = tablerock_persistence::PersistenceActor::open(&path)
        .map_err(|error| RunError::Runtime(io::Error::other(error.to_string())))?;
    let executor = EffectExecutor::new(actor, ingress);
    run_with_root_messages_and_executor(root_messages, executor).await
}

async fn run_with_root_messages_and_executor(
    root_messages: RootMessageReceiver,
    executor: EffectExecutor,
) -> Result<(), RunError> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(RunError::NonInteractive);
    }

    let mut session =
        Session::enter(io::stdout(), SessionOptions::default()).map_err(RunError::Terminal)?;
    let local = tokio::task::LocalSet::new();
    let result = local
        .run_until(async {
            #[cfg(not(test))]
            {
                run_session(&mut session, root_messages, &executor).await
            }
            #[cfg(test)]
            {
                run_session(&mut session, root_messages, &executor, &mut || Ok(())).await
            }
        })
        .await;
    finish_restoration(result, session.restore())
}

fn finish_restoration(
    result: Result<(), RunError>,
    restoration: Result<(), io::Error>,
) -> Result<(), RunError> {
    match (result, restoration) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(()), Err(restore)) => Err(RunError::Terminal(restore)),
        (Err(primary), Err(restore)) => Err(RunError::PrimaryAndRestore {
            primary: Box::new(primary),
            restore,
        }),
    }
}

async fn run_session(
    session: &mut Session<io::Stdout>,
    mut root_messages: RootMessageReceiver,
    executor: &EffectExecutor,
    #[cfg(test)] after_frame: &mut dyn FnMut() -> Result<(), RunError>,
) -> Result<(), RunError> {
    let backend = CrosstermBackend::new(session.writer_mut());
    let mut terminal = Terminal::new(backend).map_err(RunError::Terminal)?;
    let initial = terminal.size().map_err(RunError::Terminal)?;
    let mut model = Model::default();
    let bootstrap = update(
        &mut model,
        Message::Resize {
            width: initial.width,
            height: initial.height,
        },
    );
    for effect in bootstrap.effects() {
        if *effect == Effect::Exit {
            return Ok(());
        }
        executor.dispatch(effect.clone());
    }
    let mut events = EventStream::new();
    let mut root_ingress_open = true;
    let mut input = InputAdapter::default();
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);
    let mut dirty = true;
    // Continuous health for BoundedAutomatic reconnect (30s; no-op when Manual).
    let mut health_interval =
        tokio::time::interval(std::time::Duration::from_secs(30));
    health_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        if dirty {
            let mut geometry = None;
            terminal
                .draw(|frame| {
                    let area = frame.area();
                    geometry = Some(ShellView.render_with_geometry(&model, frame, area));
                })
                .map_err(RunError::Terminal)?;
            let geometry = geometry.expect("render publishes shell geometry");
            input.set_geometry(geometry.clone());
            dirty = update(&mut model, Message::FrameRendered(geometry)).needs_render();
            #[cfg(test)]
            after_frame()?;
        }

        let input_event = tokio::select! {
            biased;
            signal = &mut shutdown => {
                signal?;
                LoopInput::Signal
            }
            _ = health_interval.tick() => LoopInput::HealthTick,
            input_event = async {
                tokio::select! {
                    root = root_messages.recv(), if root_ingress_open => LoopInput::Root(root),
                    terminal = events.next() => LoopInput::Terminal(terminal),
                }
            } => input_event,
        };
        let message = match input_event {
            LoopInput::Signal => Message::Quit,
            LoopInput::HealthTick => Message::HealthTick,
            LoopInput::Root(None) => {
                root_ingress_open = false;
                continue;
            }
            LoopInput::Root(Some(Delivery::Event(message))) => message,
            LoopInput::Root(Some(Delivery::Progress(progress))) => match progress {
                #[cfg(test)]
                RootProgress::Redraw => Message::RequestRedraw,
            },
            LoopInput::Root(Some(Delivery::ResyncRequired)) => Message::EngineResyncRequired,
            LoopInput::Terminal(event) => {
                let event = event
                    .ok_or_else(|| {
                        RunError::Input(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "terminal event stream closed",
                        ))
                    })?
                    .map_err(RunError::Input)?;
                let Some(message) = input.map_backend_event_with_keymap(event, model.keymap())
                else {
                    continue;
                };
                message
            }
        };

        let result = update(&mut model, message);
        dirty |= result.needs_render();
        for effect in result.effects() {
            if *effect == Effect::Exit {
                return Ok(());
            }
            executor.dispatch(effect.clone());
        }
    }
}

async fn shutdown_signal() -> Result<(), RunError> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut terminate = signal(SignalKind::terminate()).map_err(RunError::Signal)?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result.map_err(RunError::Signal),
            _ = terminate.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.map_err(RunError::Signal)
    }
}

type PanicHook = Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Sync + Send + 'static>;

struct PanicHookGuard<'a> {
    previous: Option<PanicHook>,
    _lock: MutexGuard<'a, ()>,
}

impl<'a> PanicHookGuard<'a> {
    fn suppress(lock: MutexGuard<'a, ()>) -> Self {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        Self {
            previous: Some(previous),
            _lock: lock,
        }
    }
}

impl Drop for PanicHookGuard<'_> {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            std::panic::set_hook(previous);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::Read,
        process::Command,
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };

    use portable_pty::{CommandBuilder, PtySize, native_pty_system};

    use super::*;

    const TIMEOUT: Duration = Duration::from_secs(5);

    #[derive(Clone, Copy)]
    enum TestFault {
        ReturnedError,
        Panic,
    }

    #[test]
    fn returned_error_restores_terminal_modes_in_real_pty() {
        assert_child_restores("run::tests::returned_error_fault_child", None);
    }

    #[test]
    fn panic_restores_terminal_modes_in_real_pty() {
        assert_child_restores("run::tests::panic_fault_child", None);
    }

    #[test]
    fn busy_ingress_does_not_starve_terminal_input() {
        assert_child_restores("run::tests::busy_ingress_child", Some(b"\x03"));
    }

    #[test]
    #[ignore = "executed as a controlled PTY child"]
    fn returned_error_fault_child() {
        assert!(matches!(
            run_fault_caught(TestFault::ReturnedError),
            Err(RunError::Input(_))
        ));
    }

    #[test]
    #[ignore = "executed as a controlled PTY child"]
    fn panic_fault_child() {
        assert!(matches!(
            run_fault_caught(TestFault::Panic),
            Err(RunError::Panicked)
        ));
    }

    #[test]
    #[ignore = "executed as a controlled PTY child"]
    fn busy_ingress_child() {
        assert!(
            run_caught_boundary(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(RunError::Runtime)?
                    .block_on(async {
                        let (sender, receiver) = root_message_channel();
                        tokio::spawn(async move {
                            loop {
                                let _ = sender.publish_progress(RootProgress::Redraw);
                                tokio::task::yield_now().await;
                            }
                        });
                        run_with_root_messages(receiver).await
                    })
            })
            .is_ok()
        );
    }

    fn run_fault_caught(fault: TestFault) -> Result<(), RunError> {
        run_caught_boundary(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(RunError::Runtime)?
                .block_on(run_fault(fault))
        })
    }

    async fn run_fault(fault: TestFault) -> Result<(), RunError> {
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(RunError::NonInteractive);
        }
        let mut session =
            Session::enter(io::stdout(), SessionOptions::default()).map_err(RunError::Terminal)?;
        let (ingress, root_messages) = root_message_channel();
        let path =
            std::env::temp_dir().join(format!("tablerock-cli-fault-{}.db", std::process::id()));
        let actor = tablerock_persistence::PersistenceActor::open(&path)
            .map_err(|error| RunError::Runtime(io::Error::other(error.to_string())))?;
        let executor = EffectExecutor::new(actor, ingress);
        let local = tokio::task::LocalSet::new();
        let result = local
            .run_until(async {
                run_session(&mut session, root_messages, &executor, &mut || {
                    assert!(
                        crossterm::terminal::is_raw_mode_enabled()
                            .expect("inspect child terminal raw mode"),
                        "fault must occur only after raw mode acquisition"
                    );
                    match fault {
                        TestFault::ReturnedError => Err(RunError::Input(io::Error::other(
                            "controlled test input failure",
                        ))),
                        TestFault::Panic => panic!("controlled test panic"),
                    }
                })
                .await
            })
            .await;
        finish_restoration(result, session.restore())
    }

    fn assert_child_restores(test_name: &str, terminal_input: Option<&[u8]>) {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("open PTY");
        let executable = std::env::current_exe().expect("current test executable");
        let mut command = CommandBuilder::new(executable);
        command.args(["--exact", test_name, "--ignored", "--nocapture"]);
        command.env("TERM", "xterm-256color");
        #[cfg(unix)]
        let initial_termios = pair.master.get_termios().expect("initial PTY termios");
        let mut child = pair
            .slave
            .spawn_command(command)
            .expect("spawn fault child");
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().expect("clone PTY reader");
        let (sender, receiver) = mpsc::sync_channel(1);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let reader_thread = thread::spawn(move || {
            let mut output = Vec::new();
            let mut buffer = [0_u8; 4096];
            let mut ready = false;
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
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(error)
                        if error.raw_os_error() == Some(5)
                            || matches!(
                                error.kind(),
                                io::ErrorKind::BrokenPipe | io::ErrorKind::UnexpectedEof
                            ) =>
                    {
                        break;
                    }
                    Err(error) => panic!("read PTY output: {error}"),
                }
            }
            let _ = sender.send(output);
        });

        if let Some(input) = terminal_input {
            ready_receiver
                .recv_timeout(TIMEOUT)
                .expect("wait for first child frame");
            let mut writer = pair.master.take_writer().expect("open PTY writer");
            use std::io::Write as _;
            writer.write_all(input).expect("write terminal input");
            writer.flush().expect("flush terminal input");
        }

        let deadline = Instant::now() + TIMEOUT;
        let status = loop {
            if let Some(status) = child.try_wait().expect("poll fault child") {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().expect("kill timed-out fault child");
                panic!("fault PTY child exceeded {TIMEOUT:?}");
            }
            thread::sleep(Duration::from_millis(20));
        };
        assert!(status.success(), "fault child exited with {status:?}");
        #[cfg(unix)]
        assert_eq!(
            pair.master.get_termios().expect("restored PTY termios"),
            initial_termios,
            "raw-mode termios state must restore exactly"
        );
        drop(pair.master);
        let output = receiver.recv_timeout(TIMEOUT).expect("fault PTY output");
        reader_thread.join().expect("join PTY reader");
        crate::test_support::assert_fullscreen_lifecycle(&output);
        assert!(!String::from_utf8_lossy(&output).contains("controlled test"));
    }

    #[test]
    fn noninteractive_fault_runner_remains_rejected() {
        let output = Command::new(std::env::current_exe().expect("test executable"))
            .args([
                "--exact",
                "run::tests::returned_error_fault_child",
                "--ignored",
                "--nocapture",
            ])
            .output()
            .expect("run redirected fault child");
        assert!(!output.status.success());
    }
}
