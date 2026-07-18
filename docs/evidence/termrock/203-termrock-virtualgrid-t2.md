# TermRock T2 VirtualGrid adoption pin

Date: 2026-07-18

## Checkpoint

Plan 008. TermRock gained `VirtualGrid` at revision
`5ab74a2d03a4bec50ebe5fbc90439ae607e0215d`. TableRock pins that exact
revision. No TableRock product code uses the grid yet (plan 009).

## Decision

- Additive TermRock API: `VirtualGrid` / `VirtualGridState` with
  caller-projected resident cells, pending placeholders, two-axis viewport,
  range selection, header/cell hit regions, known/unknown totals.
- Gate mapping: keyboard/mouse/focus/empty/min-rect/Unicode covered by
  TermRock unit tests; lookbook stories `virtual-grid/basic` and
  `virtual-grid/million`; migration `0028`; public-api inventory refreshed.
- Jackin: additive only — no existing TermRock surface removed or renamed.
- TableRock pin-only change; `DataGridModel` remains plan 009.

## Bounds and failure truth

- Grid never fetches or edits; pending cells render placeholders.
- Render cost bounded by painted viewport, not dataset size.

## Evidence

- TermRock: `cargo test -p termrock --lib widgets::virtual_grid` (7 tests).
- TermRock: lookbook check green with new SVG previews.
- TableRock: `cargo test -p tablerock-tui -p tablerock-cli` after pin.

## Remaining work

- Plan 009 composes `DataGridModel` over VirtualGrid for PostgreSQL slice.
