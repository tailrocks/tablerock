# Plan 012 residual — CopyGridWindow

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| window_summary start/end/resident/totals | done |
| CopyGridWindow action | done |
| Empty resident end=start | done |
| Unit tests (grid + update) | done |

## Decision

CopyStatus is the full status line (op, bytes, sort, filters). Operators often
need only the resident page bounds for support tickets. CopyWin is a discrete
window fact line.

## Evidence

```text
cargo test -p tablerock-tui --lib window_summary_reports
cargo test -p tablerock-tui --lib copy_status_line
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for grid window copy
