# Plan 012 residual — ClearSort keeps filters

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `clear_sort` on DataGridModel | done |
| `ActionId::ClearSort` rebrowse when sort was non-empty | done |
| Toolbar Sort + ClrSort | done |
| Unit test: sort cleared, filters retained | done |

## Decision

ClearFilters still drops sort + filters + raw WHERE. ClearSort is the
narrow control so operators can drop ORDER BY without losing chip bar
state.

## Evidence

```text
cargo test -p tablerock-tui --lib clear_sort
cargo check -p tablerock-tui
```

## Remaining work

- Multi-column sort UI list (optional; CycleSort remains primary)
