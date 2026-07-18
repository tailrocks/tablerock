# Plan 012 residual — ClearRawWhere keeps chips

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ActionId::ClearRawWhere` | done |
| Clears `raw_where` only | done |
| Keeps typed filter chips | done |
| Rebrowse when cleared | done |
| No-op when already empty | done |
| Unit test | done |

## Decision

ClearFilters still wipes sort + filters + raw WHERE. ClearRawWhere is the
narrow control so operators can drop a pasted WHERE without losing chips
(mirrors ClearSort vs ClearFilters).

## Evidence

```text
cargo test -p tablerock-tui --lib edit_raw_where
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for clear raw WHERE
