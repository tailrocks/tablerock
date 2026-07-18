# Plan 009 residual — CopyStatus grid status line

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Action CopyStat | done |
| OSC 52 of `status_line()` | done |
| Unit test | done |

## Decision

Status line already aggregates operation, rows, sort, filters, staged, pk.
CopyStat pastes that redacted projection for tickets without screenshots.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_status
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for status copy
