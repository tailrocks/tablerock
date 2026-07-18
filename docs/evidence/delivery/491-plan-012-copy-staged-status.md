# Plan 012 residual — CopyStagedStatus

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Copy drafts.status_suffix trimmed | done |
| Fail closed when no staged | done |
| Action CopyStgSt | done |
| Unit test | done |

## Decision

Compact staged counts (`staged N (i↑ c· d↓)`) for tickets without the full
CopyStaged inventory panel text.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_staged_status_action
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for staged status copy
