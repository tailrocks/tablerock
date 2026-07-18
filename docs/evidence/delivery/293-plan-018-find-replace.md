# Plan 018 residual — find/replace (literal)

Date: 2026-07-18

## What landed

### QueryEditorModel
- `find_next` / `replace_next` / `replace_all`
- Literal match; optional case-insensitive
- Safety bound 10k replacements

### TUI
- `ActionId::FindReplace` → dialog
- Paste format: `find=>replace` | `find=>replace=>all` | `…=>i` for CI
- Unit: `find_and_replace_literal`, `find_replace_action_rewrites_editor_text`

## Residual

- Word-boundary and full regex modes (ledger optional polish)
- Format (pretty-print) still open

## Commands

```bash
cargo test -p tablerock-tui find_and_replace
cargo test -p tablerock-tui find_replace_action
```
