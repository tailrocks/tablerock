# Plan 012 residual — EditRawWhere dialog

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ActionId::EditRawWhere` opens confirm with existing predicate | done |
| Submit sets `grid.raw_where` and rebrowses | done |
| Empty buffer clears raw WHERE | done |
| Semicolon / multi-statement / >1024 bytes fail closed | done |
| Toolbar RawWhere | done |
| Unit test set / reject / clear | done |

## Decision

Raw WHERE is a single predicate fragment only (engine binds into browse
plan). Semicolon rejection is a simple multi-statement stop, not a full
SQL parser. Typed filter chips remain preferred for equality/null/range.

## Evidence

```text
cargo test -p tablerock-tui --lib edit_raw_where
cargo check -p tablerock-tui
```

## Remaining work

- Server-side dialect validation of raw WHERE (optional)
