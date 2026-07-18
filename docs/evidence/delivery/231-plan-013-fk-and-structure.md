# Plan 013 — FK navigation + structure facts

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `relation_foreign_keys` / `relation_column_facts` on PostgresSession | done |
| Docker: FK edge + column NOT NULL facts | done |
| FollowForeignKey action → filtered browse of referenced table | done |
| ShowStructure action → inspector column list | done |
| Bound `$n` catalog SQL in CLI (no string concat of idents) | done |
| Table ops (truncate/drop/rename gates) | open |
| Activity dashboard / cancel backends | open |

## Verification

```text
cargo test -p tablerock-tui --lib
cargo test -p tablerock-cli --lib
cargo test -p tablerock-engine --test postgres_real applies_authorized_update
```

## Provenance

`docs/product/editing.md` FK navigation; PostgreSQL catalog docs for
`pg_constraint` / `pg_attribute`. Clean-room.
