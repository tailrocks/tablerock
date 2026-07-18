# Plan 012 residual — CopySortBar

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopySortBar OSC 52 sort chips only | done |
| Empty sort no-op | done |
| Does not include filters | done |
| Action CopySort | done |
| Unit test | done |

## Decision

CopyBar joins sort + filter chip lines. Operators often paste ORDER BY
provenance alone into notes/tickets. CopySort emits `sort_chip_bar()` only.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_sort_bar_only
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for sort-bar-only copy
