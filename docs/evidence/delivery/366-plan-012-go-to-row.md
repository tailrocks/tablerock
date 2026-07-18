# Plan 012 residual — GoToRow jump

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `DataGridModel::go_to_row` sets cursor + viewport | done |
| Clamps to Exact/Estimated totals when known | done |
| Resident jump: render only (no FetchPage) | done |
| Outside resident: `FetchPage` with `next_fetch_start` | done |
| Dialog rejects non-decimal paste | done |
| Toolbar GoRow | done |
| Unit test | done |

## Decision

Absolute 0-based row index. Fetch uses existing paging pump; jump does not
load all intermediate pages into the model.

## Evidence

```text
cargo test -p tablerock-tui --lib go_to_row
cargo check -p tablerock-tui
```

## Remaining work

- Jump to last row shortcut (optional)
