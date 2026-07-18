# Plan 012 residual — CopyColumnCounts

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| column_counts_summary visible/hidden/total | done |
| Fail closed when no columns | done |
| Action CopyColN# | done |
| Unit tests | done |

## Decision

After hide/solo/invert, operators need a quick count without listing names.
CopyHid already lists hidden names; CopyColN# is the compact counts line.

## Evidence

```text
cargo test -p tablerock-tui --lib column_counts_summary
cargo test -p tablerock-tui --lib copy_status_line
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for column count copy
