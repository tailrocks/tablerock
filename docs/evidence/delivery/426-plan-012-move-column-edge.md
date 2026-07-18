# Plan 012 residual — MoveColumnFirst / MoveColumnLast

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `move_cursor_column_to_edge` first/last | done |
| Already-at-edge no-op | done |
| Actions ColHome / ColEnd | done |
| Unit test | done |

## Decision

ColL/ColR step one slot. Jumping a column to the edge of a wide layout
needs a single action; ColHome/ColEnd remove+insert at index 0 / last.

## Evidence

```text
cargo test -p tablerock-tui --lib move_cursor_column_to_edge
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for column edge move
