# TermRock migration 0024 adoption

Date: 2026-07-17

## Decision

TableRock pins TermRock `main` revision
`371ff94effaf3363c9671a7f53b0dc606f796b67` and adopts migration 0024 without a
compatibility layer.

Before, the reducer returned TermRock `UpdateResult`, the view implemented
TermRock `View`, tests used `drive_frame`, and the CLI consumed TermRock
`Dirty`. TermRock 0.11 removed those speculative APIs.

Now TableRock owns a bounded domain `Update` envelope with one optional root
effect and a render request. The shell has inherent render methods. Backend
tests call Ratatui `Terminal::draw` directly.

## Runner boundary

TermRock's closure runner owns a synchronous Crossterm event loop. TableRock's
root loop must fairly multiplex terminal input, a bounded async engine receiver,
and shutdown signals. Migration 0024 assigns external receivers and alternative
loops to consumer-owned infrastructure and permits direct `Terminal::draw`.
TableRock therefore retains its single async root multiplexer. TermRock owns
terminal lifecycle, neutral input, reusable widgets, interaction geometry, and
styling; no component owns application state or I/O.

`FrameTick` will enter the root frame when timed presentation state first lands.
Until then there is no clock read to consolidate. Future timed widgets receive
one immutable tick per frame and never read the system clock.

## Migration map

| Removed TermRock API | TableRock replacement |
|---|---|
| `UpdateResult`, `Dirty` | TableRock `Update::{needs_render,effects}` |
| `View` | inherent `ShellView::render` and `render_with_geometry` |
| `drive_frame` | Ratatui `Terminal::draw` |
| TermRock subscriptions | bounded TableRock root subscriptions |

## Evidence and provenance

- `cargo test -p tablerock-tui -p tablerock-cli --all-targets`
- TermRock migration 0024 at the pinned revision
- No external product internals or protected expression were imported.

Public source:
<https://github.com/tailrocks/termrock/blob/371ff94effaf3363c9671a7f53b0dc606f796b67/migrations/0024-v0.11.0-closure-runner-and-frame-time.md>.
