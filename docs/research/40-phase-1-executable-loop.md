# Phase 1 Executable Loop Evidence

## Checkpoint

`tablerock-cli` is now an executable terminal adapter. It explicitly rejects
non-TTY stdin/stdout, creates one Tokio runtime, enters one TermRock session,
constructs one Crossterm `EventStream`, and renders the complete root view only
when the reducer reports dirty state. Ctrl-C and Unix termination enter the same
semantic quit/effect path as the visible Quit action.

The runtime consumes the root subscription declaration and exposes an injectable
256-slot post-mapping root-message port. It is deliberately not an engine port:
the future engine adapter must first validate and map typed revisioned events.
The later bounded-ingress checkpoint in `44` adds latest progress coalescing and
explicit resync on overflow without inventing Phase 2 domain types. OS shutdown
signals are selected first and cannot be starved by ingress or terminal traffic.

The adapter maps press/repeat events to semantic focus, action-selection,
activation, resize, redraw, and quit messages. The later render-authorized
mouse, focus, and bounded-paste mapping checkpoint is recorded separately in
[`42-phase-1-render-authorized-input.md`](42-phase-1-render-authorized-input.md).

The process wrapper uses one current-thread runtime and serializes panic-hook
ownership with an RAII guard,
suppresses panic payload output while terminal state is owned, catches unwind
after Rust drops the session, restores the prior hook, and returns one fixed
safe error. Ordinary errors are fixed safe classes; their source is available
to Rust callers but the binary prints no source text, cell value, SQL, or
credential. If execution and restoration both fail, the structured error
retains both failures and makes the restoration failure observable.

## Evidence

- Pure mapping tests cover resize, focus traversal, action traversal,
  activation, Ctrl-C, repeats, releases, ordinary text, paste, and mouse.
- A real `portable-pty` harness waits for the first complete frame, drives the
  visible semantic Quit action, and proves successful exit plus terminal mode
  restoration.
- A second PTY process sends SIGTERM and proves the same successful semantic
  shutdown and restoration path.
- Redirected non-TTY execution exits with status 1, writes no stdout, and emits
  only `TableRock: interactive terminal required`.
- A process contract test fills the injected post-mapping root port to exactly
  256 messages and proves the next non-blocking send emits an explicit resync
  requirement before surviving messages.
- Workspace tests, formatting, strict Clippy, rustdoc, dependency policy, secret
  scan, and English natural-language scan are required before publication.

Later checkpoints `42`-`44` supply the input, fault-restoration, and bounded
overflow/resync evidence. Phase 1 completion remains subject to its exit audit.

External concept: async terminal event loop and PTY lifecycle verification only  
Public sources: <https://tokio.rs/>, <https://docs.rs/crossterm/0.29.0/>, <https://docs.rs/ratatui-crossterm/0.1.2/>, and <https://docs.rs/portable-pty/0.9.0/>  
TableRock requirement: Roadmap Phase 1 / delivery-plan executable terminal shell  
Implementation source: TableRock TEA contracts, official crate APIs, TermRock public session API, and independent tests  
Copied code/assets/text: none
