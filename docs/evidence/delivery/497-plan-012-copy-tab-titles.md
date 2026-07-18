# Plan 012 residual — CopyActiveTabTitle / CopyDirtyTabTitles

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyActiveTabTitle | done |
| CopyDirtyTabTitles one-per-line | done |
| Fail closed when no dirty | done |
| Actions CopyTabT / CopyDirtyT | done |
| Unit test | done |

## Decision

CopyTabs dumps the full inventory panel. Discrete title lists help tickets and
scripts without legend/markers noise.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_active_and_dirty_tab
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for tab title copy
