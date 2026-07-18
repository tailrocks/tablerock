# Plan 012 residual — HomeCursor on resident page

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `home_cursor` → start_row, col 0, viewport reset | done |
| No server I/O | done |
| Action Home | done |
| Unit test | done |

## Decision

Distinct from GoToFirstRow (absolute row 0, may fetch). Home is local:
top-left of the **resident** window only.

## Evidence

```text
cargo test -p tablerock-tui --lib home_cursor
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for resident home
