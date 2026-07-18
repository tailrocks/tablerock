# Plan 014 residual — CopyServerProgress

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyServerProgress OSC 52 when set | done |
| Missing/empty no-op | done |
| Action CopyProg | done |
| Unit test | done |

## Decision

ClickHouse X-ClickHouse-Summary progress lands on `server_progress`.
Operators need the bare progress string for tickets without status-line
parsing. CopyProg copies only that field.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_server_progress
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for progress copy
