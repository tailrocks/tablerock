# Plan 012 residual — NextDirtyTab / PrevDirtyTab

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| select_next_dirty_tab wrap | done |
| select_prev_dirty_tab wrap | done |
| Fail closed when no dirty tabs | done |
| Actions NextDirty / PrevDirty | done |
| Unit test | done |

## Decision

Operators with many tabs need to jump between dirty work without cycling clean
tabs. Wrap search; fail closed when none are dirty (including when only the
active tab is dirty — next/prev still land on it).

## Evidence

```text
cargo test -p tablerock-tui --lib select_next_prev_dirty
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for dirty-tab navigation
