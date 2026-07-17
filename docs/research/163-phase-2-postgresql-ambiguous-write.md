# Phase 2 PostgreSQL Ambiguous Write Evidence

Date: 2026-07-17

## Decision

A PostgreSQL write whose request was dispatched but whose completion was not
observed is `WriteOutcomeUnknown`. TableRock never maps that state to query
failure, success, or a retryable error. Reconnect and session recovery must not
replay it.

The Phase 2 tracer uses a fixed TableRock-owned insert delayed on the server.
The client stops awaiting the response after 100 ms without dispatching a
PostgreSQL cancel request; the server completes after 300 ms. This isolates
response uncertainty from server cancellation.

## Evidence

PostgreSQL 17.10 and 18.4 Testcontainers fixtures require:

- local response observation ends as `PostgresError::WriteOutcomeUnknown`;
- an independent session later observes exactly one durable inserted row;
- the original session drains the abandoned response and also observes exactly
  one row; and
- no automatic replay creates a second row.

The stable adapter maps this error to `AdapterFailureClass::WriteOutcomeUnknown`.
The probe table uses an identity column without a uniqueness shortcut, so a
replayed insert would increase the observed count.

## Safety contract

- Setup completes before the timed write is dispatched.
- Only explicit cancellation sends a PostgreSQL cancel request; timing out the
  response future does not claim server cancellation.
- Any local timeout after write dispatch is unknown, regardless of the later
  observed database state.
- Observation is separate evidence and never retroactively changes or retries
  the original operation.
- Stable errors and Debug output contain no SQL text, identifiers, or values.
- The tracer cannot execute presentation-supplied SQL.

## Remaining work

This closes the applied-after-timeout ambiguous-write case. Connection loss
before dispatch, during dispatch, after commit-before-response, and TLS loss
remain open. Research 164 subsequently closes transaction commit ambiguity.
Reconnect ownership, reviewed mutation plans, service/history projection,
TUI/macOS presentation, and UniFFI remain open.

## Provenance

External concepts: PostgreSQL command completion and cancellation protocol;
tokio-postgres request/response future ownership
Public sources: <https://www.postgresql.org/docs/current/protocol-flow.html>
and <https://docs.rs/tokio-postgres/0.7.18/tokio_postgres/struct.Client.html>
TableRock requirements: research 01, 06, 10, 13, 14, 20, 30, 31, 32, 84, 155
Implementation source: TableRock-owned fixed write/observation probes and tests
Copied code/assets/text: none
