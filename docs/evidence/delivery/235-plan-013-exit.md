# Plan 013 exit — PostgreSQL writes + admin (Phase 6)

Date: 2026-07-18

## Checkpoint table

| Step | Evidence |
|------|----------|
| ApplyMutations + PG executor | 227 |
| Editability + staged drafts | 228 |
| Review preview + PK identity | 229 |
| Cell edit + apply path | 230 |
| FK + structure | 231 |
| Truncate/drop + activity | 232 |
| RETURNING generated values | 233 |
| Consume-once registry clock | 234 |
| Exit / residual | this doc |

## Residual (non-blocking)

See `plans/013-postgresql-writes-and-admin.md` residual section.

## Verification (exit)

```text
cargo test -p tablerock-core --test mutation
cargo test -p tablerock-core --lib editability
cargo test -p tablerock-engine --lib postgres_mutation
cargo test -p tablerock-engine --test postgres_real applies_authorized_update
cargo test -p tablerock-tui --lib
cargo test -p tablerock-cli --lib
```
