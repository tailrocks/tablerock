# Plan 012 residual — EndCursor on resident page

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `end_cursor` → last resident row/col | done |
| No server I/O | done |
| Action End | done |
| Unit test with Home | done |

## Decision

Complement of HomeCursor. Distinct from GoToLastRow (absolute total may
fetch). End stays inside the resident window.

## Evidence

```text
cargo test -p tablerock-tui --lib home_cursor
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for resident end
