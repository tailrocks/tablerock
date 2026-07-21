# ClickHouse real-suite isolation

Date: 2026-07-22

## Attempt and invalidation

ClickHouse cancellation again requires `KILL QUERY ... SYNC` and accepts only
`kill_status=finished`, matching the Phase 2 cancellation-truth contract.

Velnor's four-way real-test execution kept multiple ClickHouse containers and
large streaming queries active while the cancellation test waited for
synchronous termination. The CI lane now runs the seven ClickHouse real tests
serially, then runs PostgreSQL, Redis, and explicit three-engine overlap at the
existing four-test bound. Cross-engine concurrency remains covered by its
purpose-built suite; cancellation proof no longer competes with unrelated
ClickHouse fixtures.

Velnor run 29853644833 disproved contention as the cause: the identical test
timed out with the ClickHouse suite running at `-j 1`. Evidence 598 records the
runtime deadlock and structural correction. The temporary serialization is
therefore removed.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p tablerock-engine --test clickhouse_real partial_rows_and_late_error_both_visible_on_one_operation -- --nocapture --test-threads=1`
  passed locally.
- `actionlint`
- Velnor run 29853644833 failed the same late-terminal test under `-j 1`.

## Provenance

No external product reference influenced this scheduling correction. The
contract remains grounded in ClickHouse's official
[`KILL` documentation](https://clickhouse.com/docs/sql-reference/statements/kill),
which distinguishes asynchronous request delivery from synchronous
termination confirmation.
