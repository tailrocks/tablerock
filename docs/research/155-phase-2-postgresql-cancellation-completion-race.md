# Phase 2 PostgreSQL Cancellation Completion Race Evidence

Date: 2026-07-17

## Decision

PostgreSQL cancellation transport success is not cancellation outcome truth.
TableRock classifies the query result after the cancel request completes:

- SQLSTATE `57014` is `ConfirmedByServer`;
- a normal result is `RequestAcceptedButQueryCompleted`;
- cancellation transport failure and unrelated query failure remain distinct
  redacted errors.

The probe uses static TableRock-owned SQL only. Cancellation never accepts SQL
text, credentials, or identifiers from presentation.

## Evidence

One shared Rust path executes both sides of the race. `SELECT pg_sleep(30)` with
cancellation after 150 ms must terminate with SQLSTATE `57014` and
`ConfirmedByServer`. `SELECT 1` with cancellation after 250 ms must complete
normally while cancellation transport succeeds, producing
`RequestAcceptedButQueryCompleted`.

The ordinary PostgreSQL 18.4 fixture proves both outcomes and a subsequent
bounded query on the same session. The custom-root required-mTLS matrix proves
both outcomes on PostgreSQL 17.10 and 18.4, then performs clean connection
shutdown. This covers plain and rustls cancellation transports and proves a late
cancel request does not poison the next operation.

This closes the Phase 2 PostgreSQL cancellation completion race. Cancellation
during page delivery, connection loss while cancelling, notices, parameters,
multiple statements, COPY, ambiguous writes, broader typed values, and
presentation remain open.

## Safety contract

- Request delivery never masquerades as server-confirmed cancellation.
- Only SQLSTATE `57014` proves server cancellation.
- A normal result remains normal even if a late cancel transport succeeds.
- No query is automatically replayed after an ambiguous transport outcome.
- Stable errors reveal no SQL text or database response detail.

## Provenance

External concepts: PostgreSQL cancellation request protocol and SQLSTATE 57014
Public sources: <https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-CANCELING-REQUESTS>
and <https://docs.rs/tokio-postgres/latest/tokio_postgres/struct.CancelToken.html>
TableRock requirements: research 01, 06, 10, 14, 20, 30, 31, 32, 81, 84, 87,
and 136
Implementation source: TableRock-owned static probes and cancellation outcome
contract
Copied code/assets/text: none
