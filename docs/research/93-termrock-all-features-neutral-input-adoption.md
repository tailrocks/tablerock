# TermRock All-Features and Neutral Input Adoption

## Decision

TableRock enables every feature published by pinned TermRock 0.10:
`crossterm` and `serde`. The feature list lives once in the workspace dependency
declaration; member crates inherit it without narrower overrides. Every future
TermRock refresh must compare the upstream `[features]` table and enable any new
published feature unless it conflicts with TableRock's fixed architecture.

`crossterm` supplies TermRock's event conversion, Ratatui backend, and scoped
terminal session. `serde` enables TermRock's owned wire/configuration surfaces
for future keymap and state persistence. TableRock still owns product state,
effects, subscriptions, persistence, and executor policy.

## Input boundary migration

Before this checkpoint, `tablerock-cli::InputAdapter` directly matched raw
Crossterm key and mouse variants. Now the CLI converts backend events through
`termrock::input::Event` immediately, and product routing consumes only
TermRock's backend-neutral key, modifier, mouse, position, resize, focus, and
unknown vocabulary.

One explicit temporary boundary remains: pinned TermRock represents bracketed
paste as the unit variant `Event::Paste` and its Crossterm adapter discards the
String. TableRock therefore intercepts only raw `crossterm::Event::Paste(text)`
at the backend boundary to preserve and bound user text, then routes all other
events through TermRock. When TermRock changes that variant to carry text,
TableRock will delete the exception and document the sequential migration; it
will not retain a compatibility path.

## Crossterm/Ratatui ownership audit

- TermRock `Session` remains the sole terminal lifecycle owner.
- Crossterm `EventStream` remains the sole backend input pump.
- `ratatui-crossterm` remains the sole terminal backend.
- TableRock keeps its one root asynchronous TEA loop because it must select
  terminal, signal, and bounded engine subscriptions. The current TermRock
  library runner does not yet own multi-source async effects.
- Widgets, theme, runtime update result, hit geometry, and neutral input come
  from TermRock. Database and product policy remain TableRock-owned.

## Verification

- `cargo tree -p termrock -e features` shows both `crossterm` and `serde`.
- CLI input/PTY tests pass after neutral conversion.
- Full workspace tests, lint, documentation, dependency, secret, English, and
  latest-pin gates are recorded in the publishing commit.

External concepts: Cargo feature unification, backend-neutral event routing
Public sources: <https://github.com/tailrocks/termrock/blob/3a80ef0c4749bd98643bcb42869293bef2cb4733/crates/termrock/Cargo.toml>, <https://github.com/tailrocks/termrock/blob/3a80ef0c4749bd98643bcb42869293bef2cb4733/crates/termrock/src/input/event.rs>
Implementation source: TableRock-owned CLI boundary and pinned TermRock contracts
Copied code/assets/text: none
