# Plan 012 residual — CopyQuickFilter

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyQuickFilter page-local text | done |
| Fail closed when empty | done |
| Action CopyPgF | done |
| Unit test | done |

## Decision

Page-local quick filter is resident-only and not in server filter chips.
Discrete copy avoids parsing status_line or filter chip bar for the needle.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_status_line
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for quick-filter copy
