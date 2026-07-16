# TermRock 0.9 Neutral Event Migration

Status: accepted and integrated on 2026-07-16.

TableRock advances its exact TermRock `main` pin from `a002902` to
`ff263f2d5fc3964d811daf5122220e6df7f95137` (`0.9.0`). TermRock's sequential
`MIGRATING.md` links the separate
`0009-v0.9.0-neutral-event-contract.md` before/after guide after `0008`.

## Old to new

TermRock removes backend event aliases, converges generic interaction outcomes,
and moves widget handlers onto state with one borrowed-data/event ordering.
Backend-neutral `termrock::input` now owns reusable event vocabulary;
`termrock::crossterm` retains terminal session lifecycle only.

TableRock already translates Crossterm input into its own root semantic TEA
messages at the CLI boundary and does not call the changed component handlers or
removed event aliases. Its only `termrock::crossterm` imports are `Session` and
`SessionOptions`, which remain canonical. Therefore migration is an exact pin
and lockfile update with no compatibility shim or competing input vocabulary.

## Verification

- Source inspection finds no removed TermRock event, handler, or outcome API.
- Root TEA input, render-authorized hit testing, and PTY lifecycle fixtures pass
  at the exact new revision.
- Workspace tests, Clippy, and rustdoc pass.

External concepts: backend-neutral component event contracts
Public source: <https://github.com/tailrocks/termrock/tree/ff263f2d5fc3964d811daf5122220e6df7f95137>
Implementation source: TableRock dependency pin only
Copied code/assets/text: none
