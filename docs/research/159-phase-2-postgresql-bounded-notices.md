# Phase 2 PostgreSQL Bounded Notice Evidence

Date: 2026-07-17

## Decision

Rust drives `tokio_postgres::Connection::poll_message` directly instead of
awaiting its `Future` implementation, because the latter only logs notices.
TableRock owns a bounded in-memory queue of 64 notices per session.

Each notice retains bounded severity, SQLSTATE, and message. Messages are capped
at 1,024 UTF-8 bytes and carry exact original byte length when truncated. Queue
overflow never blocks the connection driver and is returned explicitly as a
dropped-count delivery. Notice values are not persisted or logged by default;
`Debug` reveals lengths and truncation only.

## Evidence

Official PostgreSQL 17.10 and 18.4 Testcontainers fixtures prove:

- exact `NOTICE`, SQLSTATE `00000`, and a short ASCII message;
- a 1,200-byte multibyte message truncates at the 1,024-byte UTF-8 boundary and
  reports original length 1,200;
- `Debug` excludes the notice message;
- 70 notices without concurrent consumption retain the first 64 in order and
  return one explicit overflow delivery with exactly six dropped notices; and
- draining all retained notices permits clean session shutdown.

The same connection driver handles plain and rustls transport generically;
existing required-mTLS query/cancellation tests remain green after the ownership
change.

This closes bounded PostgreSQL notice capture and overflow semantics for Phase
2. Notice detail/hint/position fields, notification/LISTEN workflows, service and
UniFFI projection, UI presentation, persistence policy, multiple statements,
COPY, reconnect, and ambiguous writes remain open.

## Safety contract

- Notice handling cannot apply backpressure to database protocol progress.
- Overflow is explicit; no silent loss claim is allowed.
- UTF-8 truncation never splits a code point.
- Notice messages remain absent from `Debug`, errors, and default logs.
- Connection errors remain terminal after `poll_message` returns an error.

## Provenance

External concepts: tokio-postgres asynchronous notices and PostgreSQL notice
fields
Public sources: <https://docs.rs/tokio-postgres/0.7.18/tokio_postgres/struct.Connection.html#method.poll_message>
and <https://www.postgresql.org/docs/current/protocol-error-fields.html>
TableRock requirements: research 01, 06, 10, 14, 20, 30, 31, 32, 81, 84, 87,
136
Implementation source: TableRock-owned bounded notice contract and pinned
connection driver
Copied code/assets/text: none
