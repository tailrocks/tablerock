# Plan 012 residual — CursorColumnHome / CursorColumnEnd

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CursorColumnHome first visible col | done |
| CursorColumnEnd last visible col | done |
| Keep current row | done |
| No-op when already at edge | done |
| Actions ColHomeC / ColEndC | done |
| Unit tests | done |

## Decision

Home/End also reset the row to the resident page edges. Operators often need
horizontal-only edges while inspecting a mid-page row. ColHomeC/ColEndC jump
the first/last visible column only.

## Evidence

```text
cargo test -p tablerock-tui --lib jump_cursor_visible_column_edge
cargo test -p tablerock-tui --lib page_column_left_right
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for same-row column edges
