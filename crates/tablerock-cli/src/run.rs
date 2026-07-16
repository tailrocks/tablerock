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

use crate::{EventStream, InputAdapter};

/// Bounded post-mapping ingress for root subscription messages.
pub type RootMessageSender = tokio::sync::mpsc::Sender<Message>;
pub type RootMessageReceiver = tokio::sync::mpsc::Receiver<Message>;

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
    tokio::sync::mpsc::channel(capacity)
}

/// Run TableRock and contain panics after terminal restoration.
pub fn run_caught() -> Result<(), RunError> {
    static PANIC_HOOK: Mutex<()> = Mutex::new(());
    let lock = PANIC_HOOK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _hook = PanicHookGuard::suppress(lock);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(RunError::Runtime)?
            .block_on(run())
    }));
    outcome.unwrap_or(Err(RunError::Panicked))
}

/// Own the sole live terminal session and root event loop.
pub async fn run() -> Result<(), RunError> {
    let (_, root_messages) = root_message_channel();
    run_with_root_messages(root_messages).await
}

/// Run with an injected bounded stream after source-specific semantic mapping.
pub async fn run_with_root_messages(root_messages: RootMessageReceiver) -> Result<(), RunError> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(RunError::NonInteractive);
    }

    let mut session =
        Session::enter(io::stdout(), SessionOptions::default()).map_err(RunError::Terminal)?;
    let result = run_session(&mut session, root_messages).await;
    match (result, session.restore()) {
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
) -> Result<(), RunError> {
    let backend = CrosstermBackend::new(session.writer_mut());
    let mut terminal = Terminal::new(backend).map_err(RunError::Terminal)?;
    let initial = terminal.size().map_err(RunError::Terminal)?;
    let mut model = Model::default();
    let _ = update(
        &mut model,
        Message::Resize {
            width: initial.width,
            height: initial.height,
        },
    );
    let mut events = EventStream::new();
    let mut root_ingress_open = true;
    let mut input = InputAdapter::default();
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);
    let mut dirty = true;

    loop {
        if dirty {
            let mut geometry = None;
            terminal
                .draw(|frame| {
                    let area = frame.area();
                    geometry = Some(ShellView.render_with_geometry(&model, frame, area));
                })
                .map_err(RunError::Terminal)?;
            input.set_geometry(geometry.expect("render publishes shell geometry"));
            dirty = false;
        }

        let message = tokio::select! {
            biased;
            signal = &mut shutdown => {
                signal?;
                Message::Quit
            }
            root_message = root_messages.recv(), if root_ingress_open => {
                let Some(root_message) = root_message else {
                    root_ingress_open = false;
                    continue;
                };
                root_message
            }
            event = events.next() => {
                let event = event.ok_or_else(|| {
                    RunError::Input(io::Error::new(io::ErrorKind::UnexpectedEof, "terminal event stream closed"))
                })?.map_err(RunError::Input)?;
                let Some(message) = input.map_event(event) else {
                    continue;
                };
                message
            }
        };

        let result = update(&mut model, message);
        dirty |= result.is_dirty();
        if result.effects().contains(&Effect::Exit) {
            return Ok(());
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
