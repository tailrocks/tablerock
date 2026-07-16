# Phase 1 Terminal Lifecycle Evidence

## Checkpoint

TableRock advances its exact TermRock pin to compatibility evidence revision
[`9099b3db`](https://github.com/tailrocks/termrock/commit/9099b3db0c3318fd183d076c4e8f8002a877be6a).
Its lifecycle implementation revision `824783b0` makes a full-screen session the sole
owner of raw mode, alternate screen, mouse capture, bracketed paste, line
wrapping, and cursor visibility.

Cleanup obligations are armed before every fallible acquisition, so a command
that writes partially or fails while flushing still receives its safe inverse.
Restoration continues after an error, preserves the earliest error, keeps failed
cleanup armed for retry, executes in exact reverse order, and is idempotent.
Cursor and line-wrap behavior remain private consequences of full-screen
ownership rather than new caller policy flags.

## Evidence

- Seven focused TermRock session tests cover exact ordering, every partial-write
  and acquisition-flush boundary, post-write flush failure, failed-cleanup
  retry, two-failure first-error identity, and idempotence.
- The full TermRock workspace has 224 passing tests; strict Clippy, no-feature
  and example builds, rustdoc, dependency policy, secret scan, and 196/196
  semver checks pass.
- Jackin `27c450e9` compiles across all workspace targets with a temporary exact
  pin to implementation revision `824783b0`; its files were restored clean.
- TableRock's real PTY normal-exit and SIGTERM tests now require line-wrap,
  cursor, paste, mouse, and alternate-screen restoration sequences.

Later evidence now covers render-authorized input in `42` and returned-error and
panic PTY restoration, including raw-termios inspection, in `43`.
Bounded overflow/resync is recorded in `44`; the completed exit audit is `45`.

External concept: scoped terminal mode ownership only  
Public source: <https://github.com/tailrocks/termrock/tree/9099b3db0c3318fd183d076c4e8f8002a877be6a>  
TableRock requirement: Roadmap Phase 1 / delivery-plan terminal lifecycle  
Implementation source: TableRock lifecycle requirements, Crossterm commands, TermRock neutral API, and independent tests  
Copied code/assets/text: none
