# Plan 012 residual — CopyPreviewTabTitles

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyPreviewTabTitles one-per-line | done |
| Fail closed when none preview | done |
| Action CopyPrevT | done |
| Unit test | done |

## Decision

Preview tabs are disposable browse peeks. Discrete list for tickets/scripts
without full inventory. Completes dirty/preview title pair (CopyDirtyT).

## Evidence

```text
cargo test -p tablerock-tui --lib copy_active_and_dirty_tab
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for preview tab title copy
