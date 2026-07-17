# TermRock 0.9 input and OSC migration

Status: accepted and integrated on 2026-07-16.

TableRock pins TermRock `main` commit
`2f441cc00eff7caecedd49a368aec6aa349d7bc1` (`0.9.0`). This supersedes the
integration pin recorded in `57-termrock-0.9-migration.md` without rewriting
that historical checkpoint.

## Sequential upstream migrations

TermRock's `MIGRATING.md` links one separate before/after document for each
incompatible change:

1. `0004-v0.9.0-typed-osc-requests.md` replaces free-form clipboard selection
   strings with `ClipboardSelection`, validates hyperlink schemes and terminal
   controls, and bounds clipboard source text at 100,000 bytes.
2. `0005-v0.9.0-unknown-key-handling.md` adds `KeyCode::Unknown` instead of
   collapsing unsupported backend keys into Escape, and makes releases
   non-actionable in reusable widgets.

## TableRock adaptation

Before this pin, TableRock consumed neither OSC request construction nor an
exhaustive match over TermRock `KeyCode`. After this pin, those boundaries
remain unused: TableRock's Crossterm adapter maps its own closed input event
contract, and rendering uses no OSC requests. No compatibility facade or stale
parallel approach was added.

Future use must adopt the typed clipboard selection, handle rejection as no
output, preserve unknown keys as non-actions, and pass key kinds unchanged so
TermRock widgets own release filtering.

## Provenance

External concepts: typed terminal requests, closed neutral key vocabulary,
release filtering.

Public source: <https://github.com/tailrocks/termrock/tree/2f441cc00eff7caecedd49a368aec6aa349d7bc1>

Implementation source: no TableRock behavior copied or translated; this
checkpoint only adapts the exact dependency pin and records its public API
migration requirements.
