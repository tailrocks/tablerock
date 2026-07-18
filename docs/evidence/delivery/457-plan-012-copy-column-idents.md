# Plan 012 residual — CopyColumnIdents

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyColumnIdents visible columns quoted | done |
| Comma-separated list | done |
| Respects Solo/hide layout | done |
| Action CopyColsQ | done |
| Unit test | done |

## Decision

CopyCols is raw TSV headers. SQL SELECT lists need `"a", "b", "c"`.
CopyColsQ quotes visible columns only (layout-aware).

## Evidence

```text
cargo test -p tablerock-tui --lib copy_column_idents_visible
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for visible column SQL ident list
