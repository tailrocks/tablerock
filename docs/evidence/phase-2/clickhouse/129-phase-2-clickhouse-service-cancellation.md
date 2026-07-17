# Phase 2 ClickHouse service cancellation evidence

Date: 2026-07-17

ClickHouse query-ID cancellation now crosses the complete
core/service/runtime/adapter path. Each owned ClickHouse session records only
one active `query_id`, capped at 256 bytes; query text remains inside the
transient client request. A second stream cannot overwrite that cancellation
target. Cancellation uses a separate HTTP request with a bound server parameter:

```sql
KILL QUERY WHERE query_id = {target:String} SYNC
```

The cancellation request reads only the first bounded RowBinary field from the
response. `kill_status=finished` records server confirmation. An empty result or
another status becomes `ServerRejected`; a client/protocol failure becomes
`TransportFailed`. Neither result is mislabeled as successful dispatch.

After `finished`, shared session state lets the original result stream map its
terminal client error to `ServerConfirmedCancelled`. The runtime still reports
`RequestSent` separately before the terminal outcome, preserving the same
request-versus-outcome model used for PostgreSQL. Dropping a result stream
clears its matching active query identity without retaining SQL or result data.

One real Testcontainers contract runs on ClickHouse 25.8 and 26.3 LTS with
both uncompressed and LZ4 requests. It waits for a bounded page from a long
stream, requests cancellation, tolerates already-buffered bounded pages, then
requires `RequestSent` and `ServerConfirmedCancelled` within five-second
deadlines for all four combinations.

This follows the official
[`KILL QUERY`](https://clickhouse.com/docs/sql-reference/statements/kill)
contract: `SYNC` waits for termination and reports `finished`, while the
default asynchronous form would prove only signal dispatch. The official
[`clickhouse-rs` 0.15.1](https://github.com/ClickHouse/clickhouse-rs)
client source establishes bound parameters, per-query settings, and streaming
raw result APIs. Context7 was requested first but unavailable because its
monthly quota was exhausted.

Redis client-stop/post-dispatch truth remains required. No external-product
source or protected expression influenced this checkpoint.
