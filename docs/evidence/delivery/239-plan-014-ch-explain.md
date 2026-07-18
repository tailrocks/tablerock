# Plan 014 — ClickHouse EXPLAIN raw + structured

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `explain_raw` | done |
| `explain_structured` (AST or unknown-node fallback) | done |
| Docker SELECT 1 explain | done |
| Editor Explain action UI | open |

## Verification

```text
cargo test -p tablerock-engine --test clickhouse_real explain_raw
```
