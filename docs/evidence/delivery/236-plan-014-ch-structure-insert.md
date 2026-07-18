# Plan 014 — ClickHouse structure facts + progressive INSERT

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `relation_engine_facts` / `relation_column_facts` (named params) | done |
| `ClickHouseSession::apply_authorized_mutation` progressive INSERT | done |
| UPDATE/DELETE fail closed with non-transactional wording | done |
| Docker: MergeTree facts + INSERT apply + UPDATE reject | done |
| TLS custom CA / progress events / async mutations / cancel UI | open |

## Verification

```text
cargo test -p tablerock-engine --lib clickhouse_mutation
cargo test -p tablerock-engine --test clickhouse_real structure_facts_and_progressive_insert
```

## Provenance

`docs/product/clickhouse.md` writes + structure; official ClickHouse
`system.tables` / `system.columns`; plan 013 mutation typestate. Clean-room.
