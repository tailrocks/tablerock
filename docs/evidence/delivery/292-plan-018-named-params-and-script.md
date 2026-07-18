# Plan 018 residual — named parameters + multi-statement script

Date: 2026-07-18

## Named parameters

### Core (`tablerock_core::named_params`)
- `rewrite_named_params` — `:name` → `$n` (same name reuses index)
- Skips strings, comments, dollar-quotes; ignores `::cast` and `:=`
- `parse_param_bindings` / `bind_named_values`
- Values **never** concatenated into SQL

### Engine
- `FilterValue::Null` + `parse_bind_text` heuristics
- PostgreSQL `stream_statement` binds Null as `Option::<String>::None`

### TUI
- Run with unbound `:name` → `ConfirmDialog::BindParams`
- Paste `name=value;…` → `ExecuteSql` with rewritten SQL + positional texts
- Unit: `run_sql_with_named_params_opens_bind_dialog`

## Multi-statement result sections

- Explicit multi-span **selection** → `Effect::ExecuteSqlScript`
- CLI runs statements in order; partial failure keeps earlier sections
- `ScriptSections` inspector lines; last grid page still delivered
- Default Run (no selection) remains current-statement only
- Unit: `run_sql_multi_statement_selection_emits_script_effect`

## Commands

```bash
cargo test -p tablerock-core named_params
cargo test -p tablerock-engine parse_bind
cargo test -p tablerock-tui run_sql
cargo check -p tablerock-cli
```
