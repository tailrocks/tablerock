# Plan 012 residual — RemoveFirstFilter

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `remove_first_filter` drops oldest chip | done |
| Empty no-op | done |
| Action ShiftFilt + rebrowse | done |
| Unit test | done |

## Decision

PopFilt removes the newest chip. Multi-chip refinement often wants FIFO
undo of the earliest constraint. ShiftFilt removes filters[0].

## Evidence

```text
cargo test -p tablerock-tui --lib remove_last_and_column
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for first-filter remove
