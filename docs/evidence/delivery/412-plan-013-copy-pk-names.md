# Plan 013 residual — CopyPkNames

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Action CopyPk | done |
| TSV of `identity_columns` | done |
| No-op when empty | done |
| Unit test | done |

## Decision

Multi-column PKs need pasteable locator names for SQL/docs. Complements
status-line `pk …` projection (382).

## Evidence

```text
cargo test -p tablerock-tui --lib copy_pk
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for pk name copy
