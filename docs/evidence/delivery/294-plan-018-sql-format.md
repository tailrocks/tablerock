# Plan 018 residual — SQL format action

Date: 2026-07-18

## What landed

### Core `format_sql`
- Uppercases fixed keyword set as whole words
- Collapses horizontal whitespace outside literals
- Preserves strings, identifiers, dollar-quotes, line/block comments
- Unit: keyword case, string preserve, comment preserve

### TUI
- `ActionId::FormatSql` rewrites active editor buffer
- Unit: `format_sql_action_uppercases_keywords`

## Residual

- Richer pretty-print (clause newlines, indent) optional polish
- Type-specific cell editors still open

## Commands

```bash
cargo test -p tablerock-core sql_format
cargo test -p tablerock-tui format_sql_action
```
