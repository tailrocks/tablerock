# Concurrent cancellation transport and stream drain

Date: 2026-07-22

## Root cause

Velnor run 29853644833 disproved ClickHouse fixture concurrency as the cause of
the cancellation timeout. The engine runtime awaited the driver's synchronous
cancellation request inline. During that await it stopped polling the original
HTTP result stream. A remote Docker path can apply response backpressure, while
ClickHouse `KILL QUERY ... SYNC` waits for the original query to terminate;
each side therefore waited for progress owned by the other.

The runtime architecture permitted the bug because cancellation transport and
result-stream progress shared one awaited control path despite having a bounded
task owner capable of supervising both.

## Correction

After stream creation, the runtime starts at most one owned cancellation task
and continues selecting over stream pages, control requests, result-event
capacity, and cancellation completion. The task returns its dispatch fact
through a capacity-one channel. Operation exit aborts and joins any unfinished
cancel task, so no detached work survives ownership teardown.

Concurrent cancellation is an explicit driver capability; PostgreSQL and Redis
retain their existing inline dispatch ordering. The driver contract also has a
synchronous, I/O-free `prepare_cancel`
hook. ClickHouse uses it to mark the active stream before the transport task is
scheduled, closing the race where a killed stream could observe EOF and erase
its query identity first. The stream retains confirmed state through EOF,
waits for the in-flight synchronous kill resolution, then emits
`ServerConfirmedCancelled`; normal completion still clears state immediately.

A deterministic coupled fake makes cancellation wait for continued stream
polling. It deadlocks under the former architecture and now proves dispatch,
terminal delivery, and bounded join.

## Verification

- `cargo test -p tablerock-engine --test driver_runtime drains_stream_while_cancel_transport_waits_for_it`
  passed.
- Complete `driver_runtime` suite passed: 4 tests.
- ClickHouse full two-version/two-compression stream/cancel matrix passed.
- ClickHouse late-terminal real test passed locally.
- Redis full supported-version/protocol matrix test passed after confirming
  inline cancellation remains unchanged.
- `cargo fmt --all -- --check`
- `actionlint`
- Velnor hosted rerun required after push.

## Provenance

No external product reference influenced this runtime correction. The
ClickHouse cancellation requirement remains based on official
[`KILL QUERY`](https://clickhouse.com/docs/sql-reference/statements/kill)
semantics. Architecture and regression harness are TableRock-owned.
