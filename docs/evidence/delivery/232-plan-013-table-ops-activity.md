# Plan 013 — Table ops gates + activity snapshot

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Truncate/Drop confirm dialogs require exact table name paste | done |
| Fail closed: wrong name never dispatches effect (reducer test) | done |
| `ExecuteTableOp` with fixed op vocabulary + `quote_ident` only | done |
| `LoadActivity` / `pg_stat_activity` snapshot in inspector | done |
| Permission-aware cancel/terminate backends | done (evidence 327) |
| Rename table gate | done (evidence 340) |

## Verification

```text
cargo test -p tablerock-tui --lib truncate_confirm
cargo test -p tablerock-tui --lib
cargo test -p tablerock-cli --lib
```

## Provenance

`docs/product/editing.md` destructive ops gates; PostgreSQL `TRUNCATE` /
`DROP TABLE` / `pg_stat_activity` docs. Clean-room.
