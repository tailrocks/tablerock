# Plan 012 residual — CopyRow CSV / JSON / Markdown

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyRow remains TSV default | done |
| CopyRowCsv / CopyRowJson / CopyRowMarkdown | done |
| Reuses `format_copy(…, CopyScope::Row, …)` | done |
| Status shows format label + byte count | done |
| Unit test | done |

## Decision

Loaded-result copy already exposes six formats. Row scope now has the common
document formats as first-class actions without a format picker dialog.
SQL INSERT/UPDATE for a single row remain available via the existing
CopySqlInsert/CopySqlUpdate loaded-result path when identity is proven.

## Evidence

```text
cargo test -p tablerock-tui --lib format_cursor_row
cargo test -p tablerock-tui --lib
```

## Remaining work

- Single picker dialog — shipped as evidence 393
