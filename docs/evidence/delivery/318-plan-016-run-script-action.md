# Plan 016 residual — RunScript full-buffer multi-statement

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ActionId::RunScript` | done |
| Workbench action bar “Script” | done |
| Full-buffer selection forced before `ExecuteSqlScript` | done |
| Redis RunScript → sequential pipeline (same as Run) | done |
| Default Run stays current-statement / multi-span selection | done (unchanged) |
| Unit: 3 statements without prior selection | done |

## Decision

Default Run remains current-statement (or multi-span selection → script).
Explicit **Script** runs the entire editor buffer as an ordered multi-statement
script with result sections, without requiring the operator to select spans
first. Closes plan 016 “Multi-statement UI wiring into QueryEditorModel run
path” residual (selection path was evidence 292).

## Evidence

```text
cargo test -p tablerock-tui --lib run_script_action
cargo test -p tablerock-tui --lib run_sql_multi_statement
```

## Remaining work

- Fuzzy multi-preset filter picker polish
