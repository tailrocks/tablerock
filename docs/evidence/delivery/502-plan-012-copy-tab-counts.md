# Plan 012 residual — CopyTabCounts

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| tab_counts_summary total/dirty/preview/running | done |
| Fail closed when no tabs | done |
| Action CopyTabN# | done |
| Unit tests | done |

## Decision

Compact strip health without the full CopyTabs inventory/legend. Mirrors
CopyColN# for columns (evidence 500).

## Evidence

```text
cargo test -p tablerock-tui --lib tab_counts_summary
cargo test -p tablerock-tui --lib copy_active_and_dirty_tab
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for tab count copy
