# ClickHouse asynchronous cancellation dispatch

Date: 2026-07-22

## Failure

After container reachability was corrected, Velnor run 29853060077 passed its
first three ClickHouse tests. The partial-page cancellation test then waited
past 15 seconds for a terminal event. `dispatch_cancel()` issued `KILL QUERY
... SYNC`, coupling request dispatch to completion of server termination and
blocking the engine runtime's cancellation event pump under runner load.

## Correction

Cancellation now issues `KILL QUERY ... ASYNC`. ClickHouse's documented
default asynchronous form sends the termination request without waiting for
the query to stop. Both `waiting` and `finished` response states prove that the
request matched and was accepted; unrelated states remain rejected. Stream
termination remains a separate later event, preserving TableRock's distinction
between dispatch and terminal outcome.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p tablerock-engine --lib asynchronous_kill_status_distinguishes_acceptance`
  passed: 1 passed, 112 filtered out.
- `cargo test -p tablerock-engine --test clickhouse_real partial_rows_and_late_error_both_visible_on_one_operation -- --nocapture --test-threads=1`
  passed: 1 passed, 6 filtered out.
- Velnor hosted rerun required after push.

## Provenance

No external product reference influenced this fix. ClickHouse's official
[`KILL` statement documentation](https://clickhouse.com/docs/sql-reference/statements/kill)
defines `ASYNC` as non-waiting dispatch, `SYNC` as waiting for termination, and
the `waiting`/`finished` response states. Implementation and tests remain
TableRock-owned.
