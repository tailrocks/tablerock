# Plan 018 residual — EXPLAIN text tree projection

Date: 2026-07-18

## What landed

- `InspectorModel::from_explain_text`
- `explain_tree_lines` — 2-space indent → `│` / `└─` glyphs
- Auto-detect plan-like text in `lines()` via cost/scan markers
- Unit: `explain_tree_uses_indent_glyphs`

## Commands

```bash
cargo test -p tablerock-tui explain_tree
```

## Residual

- Full structured JSON plan parsers per engine still optional polish
