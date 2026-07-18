# Plan 014 residual — partial rows + late terminal on one CH operation

Date: 2026-07-18

## Proof

`crates/tablerock-engine/tests/clickhouse_real.rs::partial_rows_and_late_error_both_visible_on_one_operation`

1. Submit `CancellationStream` with `max_rows=1` through `EngineService`
2. Observe `Page` with ≥1 row and partial delivery / row-limit warning
3. Request cancel
4. Observe late `Terminal` (cancel/fail/unknown class)
5. Assert the first page still owns its row bytes after the terminal

```bash
cargo test -p tablerock-engine --test clickhouse_real partial_rows_and_late
# 1 passed
```
