# DataGridModel, VirtualGrid paint, and quote_ident

Date: 2026-07-18

## Checkpoint

Plan 009 step 1. Presentation-local `DataGridModel` with distinction classes
for every ValueKind class, resident-window pending cells, and status facts.
Workbench active tab paints through TermRock `VirtualGrid`. Engine
`quote_ident` / `qualify_table` for safe identifier SQL assembly.

## Decision

- TUI stays free of `tablerock-core` page types: CLI will project cells later.
- Distinctions use glyph+text, never color alone.
- `quote_ident` doubles internal `"` and rejects empty/NUL.

## Evidence

- `model::grid::tests::*`
- `ident::tests::*` (engine lib)
- `cargo test -p tablerock-tui -p tablerock-engine --lib`
- Log: implementer `grid-ident-tests.log`

## Remaining work

- Browse table Execute + ResultStore admit + FetchPage scroll.
- SQL tab + cancel + inspector.
- Phase 4 exit evidence.
