# Plan 012 residual — ClearFiltersKeepSort

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `clear_filters_keep_sort` drops filters + raw WHERE | done |
| Sort keys preserved | done |
| Empty no-op | done |
| Action ClrFiltS + rebrowse | done |
| Palette surfaces ClrFilt + ClrFiltS | done |
| Unit test | done |

## Decision

ClearFilters historically clears sort + filters + raw (full server
controls). Operators often want filter wipe while keeping multi-key ORDER
BY. ClrFiltS is filters/raw only. ClrFilt also added to the action palette
(was previously action-only).

## Evidence

```text
cargo test -p tablerock-tui --lib clear_filters_keep_sort
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for keep-sort filter clear
