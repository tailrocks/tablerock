# Plan 013 residual — CopyExistsSql / CopyDeleteWhereSql

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyExistsSql nested SELECT 1 + locator | done |
| CopyDeleteWhereSql DELETE + locator WHERE | done |
| Fail closed without identity WHERE | done |
| Never emits bare DELETE | done |
| Actions CopyExists / CopyDelW | done |
| Unit test | done |

## Decision

Presentation-only scaffolds. EXISTS is a common presence check. DELETE
copy requires the same identity WHERE as mutations; without locator the
action is a no-op so operators cannot paste an unqualified DELETE.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_exists_and_delete_where
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for EXISTS / DELETE WHERE scaffolds
