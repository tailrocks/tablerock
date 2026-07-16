# Phase 1 Render-Authorized Input Evidence

## Checkpoint

The CLI maps Crossterm focus, paste, and pointer facts into root TEA messages.
Pointer coordinates never cross the adapter boundary. Each completed render
returns immutable hit geometry derived from the regions actually painted by
TermRock widgets; the adapter resolves coordinates against only that current
geometry and sends stable focus or action identifiers to the reducer.

Primary-button press, drag, release, movement, and four-axis wheel input have
explicit semantic messages. Action activation requires press and release over
the same painted action. Other mouse buttons remain unowned. Focus loss clears
transient hover and press state so a later release cannot activate stale UI.

Paste is bounded to 1 MiB on ingress at a UTF-8 boundary. Its custom debug
representation exposes only byte count and truncation state. The empty shell
does not retain paste content; future editors must consume it in their own
focused reducer path.

## Evidence

- Adapter tests render the shell, install the returned geometry, and prove
  pointer mapping to action and content identifiers.
- Reducer tests prove matching press/release activation, mismatched-release
  rejection, focus-loss cleanup, and non-retention of paste.
- Geometry tests prove wide-layout targets and an empty hit map for the
  too-small projection.
- Paste tests prove secret-text debug redaction, a fixed byte bound, UTF-8-safe
  truncation, and explicit truncation reporting.
- A real PTY resizes from 80x24 to 100x30, sends focus loss/gain and a private
  bracketed paste, then activates Open through SGR mouse coordinates resolved
  from the resized frame. Keyboard selection exits only after that mouse action
  establishes action focus. The private paste is absent from captured output,
  and terminal modes restore completely.
- Workspace tests, formatting, strict Clippy, rustdoc, dependency policy,
  secret scanning, and English natural-language scanning gate publication.

Returned-error/panic PTY restoration and typed engine-event
overflow/resynchronization remain part of the Phase 1 exit checkpoint.

External concept: terminal focus, bracketed-paste, and mouse event semantics only  
Public source: <https://docs.rs/crossterm/0.29.0/crossterm/event/enum.Event.html>  
TableRock requirement: Roadmap Phase 1 and quality plan input parity rows  
Implementation source: TableRock TEA contracts, TermRock public hit regions,
Crossterm public events, and independent tests  
Copied code/assets/text: none
