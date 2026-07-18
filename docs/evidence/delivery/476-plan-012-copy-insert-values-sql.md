# Plan 012 residual — CopyInsertSql / CopyValuesSql

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyInsertSql identity-gated scaffold | done |
| CopyValuesSql cursor-row literals | done |
| Visible columns only | done |
| Pending cells fail closed | done |
| Actions CopyIns / CopyVals | done |
| Pure formatter + unit tests | done |

## Decision

Operators paste INSERT scaffolds into the SQL editor without retyping quoted
idents. CopyIns needs base-table identity; CopyVals is a presentation VALUES
tuple for the cursor row (composable after CopyIns). Neither executes.

## Evidence

```text
cargo test -p tablerock-tui --lib format_insert_and_values
cargo test -p tablerock-tui --lib copy_insert_and_values
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for INSERT/VALUES scaffolds
