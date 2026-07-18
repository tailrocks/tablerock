# Plan 013 residual — CopyWhere SQL fragment

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `cursor_where_sql` → `WHERE "col" = '…'` | done |
| Quoted identifiers, escaped literals, NULL keyword | done |
| Action CopyWhere | done |
| Presentation only (never executed) | done |
| Unit test | done |

## Decision

Clipboard aid for ad-hoc SQL. Not a substitute for parameterized mutations;
operators paste into external clients at their own risk. Values from
presentation text, not re-decoded binary.

## Evidence

```text
cargo test -p tablerock-tui --lib cursor_locator
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for WHERE paste aid
