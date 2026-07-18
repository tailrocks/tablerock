# Plan 014 residual — ClickHouse OPTIMIZE TABLE gate

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ActionId::OptimizeTable` + exact-name confirm | done |
| CLI `OPTIMIZE TABLE db.table` via `quote_ident` only | done |
| ClickHouse-only; other engines fail closed for optimize | done |
| PG table ops unchanged (truncate/drop/vacuum/analyze/rename) | done |
| Unit test for confirm → `ExecuteTableOp { op: optimize }` | done |

## Decision

OPTIMIZE is ClickHouse maintenance counterpart to PG VACUUM. Schema field
holds the database name (existing CH browse identity). No FINAL/PARTITION
options in v1.

## Evidence

```text
cargo test -p tablerock-tui --lib optimize_table
cargo check -p tablerock-cli
```

## Remaining work

- Optional FINAL / partition-scoped optimize
