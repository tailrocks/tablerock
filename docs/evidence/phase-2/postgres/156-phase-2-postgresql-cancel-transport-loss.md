# Phase 2 PostgreSQL Cancel Transport Loss Evidence

Date: 2026-07-17

## Decision

When PostgreSQL disappears before the separate cancel connection can deliver
its request, TableRock reports `CancellationTransport`. It does not infer that
the server cancelled the query from the original query connection also failing.
The subsequent session shutdown independently reports `Connection`.

Cancellation-transport failure takes precedence over an unclassified target
query disconnect because it is the only proven cancel-dispatch fact. No retry or
query replay occurs.

## Evidence

Testcontainers Rust runs official PostgreSQL 17.10 and 18.4. Each case starts a
30-second read-only sleep and schedules cancellation after one second. After 50 ms,
the test force-stops the server and waits for both paths. The cancellation
connection cannot open and must return `PostgresError::CancellationTransport`.
Consuming the session then requires `PostgresError::Connection` from the driven
connection task.

This closes cancellation transport loss before request delivery on both pinned
lines. Loss after the cancel packet is delivered but before query outcome,
page-delivery cancellation races, reconnect policy, ambiguous writes, notices,
parameters, multiple statements, COPY, broader typed values, and presentation
remain open.

## Safety contract

- Query disconnect is never upgraded to server-confirmed cancellation.
- Only SQLSTATE `57014` proves server cancellation.
- Failed cancel dispatch is not automatically retried.
- The disconnected session is terminal and cannot replay work.
- Stable errors expose no SQL, endpoint, credential, or server response detail.

## Provenance

External concepts: PostgreSQL separate cancellation connection and connection
shutdown
Public sources: <https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-CANCELING-REQUESTS>
and <https://docs.rs/tokio-postgres/latest/tokio_postgres/struct.CancelToken.html>
TableRock requirements: research 01, 06, 10, 14, 20, 30, 31, 32, 81, 84, 136,
and 155
Implementation source: TableRock-owned cancellation outcome contract and
Testcontainers failure injection
Copied code/assets/text: none
