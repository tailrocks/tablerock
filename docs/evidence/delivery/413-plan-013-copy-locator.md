# Plan 013 residual — CopyLocator for cursor row

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `cursor_locator_text` → `col=value` lines | done |
| Action CopyLoc | done |
| No-op without identity columns or values | done |
| Unit test | done |

## Decision

Pasteable locator for WHERE clauses and support tickets. Uses the same
`locator_for_row` facts as mutations.

## Evidence

```text
cargo test -p tablerock-tui --lib cursor_locator
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for locator copy
