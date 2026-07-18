# Plan 014 — ClickHouse async UPDATE/DELETE mutations

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| ALTER TABLE UPDATE/DELETE via authorized plan | done |
| `returned` markers: kind=async_mutation_*, transactional=false | done |
| `latest_mutation_status` polls `system.mutations` | done |
| Docker: mutation accepted + poll is_done | done |
| KILL MUTATION gate / unknown on connection loss | open |
| Progress/query-id UI + TLS CA | open |

## Verification

```text
cargo test -p tablerock-engine --lib clickhouse_mutation
cargo test -p tablerock-engine --test clickhouse_real structure_facts_and_progressive_insert
```
