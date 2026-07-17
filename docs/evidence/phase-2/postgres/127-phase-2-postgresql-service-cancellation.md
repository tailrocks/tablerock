# Phase 2 PostgreSQL service cancellation evidence

Date: 2026-07-17

PostgreSQL cancellation now crosses the complete core/service/runtime/adapter
path without inventing server truth. A core cancel request first moves the
operation to `CancelRequested`. The runtime then asks the owned PostgreSQL
session to send a fresh-protocol cancellation request through its
`tokio-postgres` cancel token. Dispatch reports exactly one of:

- `Unsupported`: the adapter has no server cancellation mechanism;
- `RequestSent`: the cancellation transport accepted the request;
- `TransportFailed`: the cancellation request could not be delivered;
- `ServerRejected`: a synchronous server response did not confirm a target.

`RequestSent` is not terminal success. Only PostgreSQL SQLSTATE `57014`
observed from the running row stream becomes
`ServerConfirmedCancelled`. A completed query after a cancel request remains
`CompletedBeforeCancel`; other errors remain failures.

The runtime now emits `Started` when the owned task begins and multiplexes
stream creation, cancellation, client stop, and bounded event delivery. This
removes the former blind interval where a query blocked inside stream creation
before runtime cancellation could be handled. A deterministic unit fixture
proves cancellation during stream creation. A PostgreSQL 18.4 Testcontainers
fixture proves a real delayed query reports `RequestSent` and then
`ServerConfirmedCancelled`, each within a five-second test deadline.

ClickHouse query-ID cancellation is subsequently proven in
[`129-phase-2-clickhouse-service-cancellation.md`](../clickhouse/129-phase-2-clickhouse-service-cancellation.md).
Redis client-stop/post-dispatch truth remains required. PostgreSQL confirmation
is not generalized to engines whose protocols provide different evidence.

Sources are TableRock-owned requirements, `tokio-postgres` behavior, PostgreSQL
SQLSTATE behavior, and direct pinned-server tests. No external-product source
or protected expression influenced this checkpoint.
