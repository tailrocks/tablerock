# Plan 013 residual — rename table gate

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ActionId::RenameTable` opens confirm with new-name buffer | done (prior) |
| Empty / same-name submit does not dispatch | done (prior + test) |
| Submit with new name → `ExecuteTableOp { op: rename }` | done (prior + test) |
| CLI builds `ALTER TABLE … RENAME TO` via `quote_ident` only | done (prior) |
| Unit test `rename_table_confirm_emits_execute_table_op` | done |

## Decision

Rename is less destructive than truncate/drop: operator types the *new*
identifier rather than re-typing the old name. Same-name and empty reject
pre-effect. SQL remains fixed vocabulary + quoted identifiers.

## Evidence

```text
cargo test -p tablerock-tui --lib rename_table
```

## Remaining work

- Maintenance/optimize ops (VACUUM/ANALYZE gates) if product requires
