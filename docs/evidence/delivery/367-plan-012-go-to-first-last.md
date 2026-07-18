# Plan 012 residual — GoToFirstRow / GoToLastRow

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `GoToFirstRow` → jump 0 | done |
| `GoToLastRow` → Exact/Estimated last index | done |
| Shared `jump_to_row` with GoToRow dialog | done |
| Fetch when last outside resident | done |
| Toolbar First / Last | done |
| Unit test extended on go_to_row | done |

## Decision

Last uses totals when known; otherwise end of resident window. Same fetch
rules as GoToRow.

## Evidence

```text
cargo test -p tablerock-tui --lib go_to_row
cargo check -p tablerock-tui
```

## Remaining work

- None material for first/last shortcuts
