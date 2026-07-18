# Plan 012 residual — FirstDirtyTab / LastDirtyTab

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| select_first_dirty_tab | done |
| select_last_dirty_tab | done |
| Fail closed when none dirty | done |
| Actions FirstDirty / LastDirty | done |
| Unit test | done |

## Decision

Next/Prev cycle dirty tabs; first/last jump to strip ends without cycling.
Mirrors FirstStg/LastStg for staged cells (evidence 494).

## Evidence

```text
cargo test -p tablerock-tui --lib select_next_prev_dirty
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for first/last dirty tab jump
