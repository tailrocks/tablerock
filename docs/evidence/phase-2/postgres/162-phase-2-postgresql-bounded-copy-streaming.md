# Phase 2 PostgreSQL Bounded COPY Streaming Evidence

Date: 2026-07-17

## Decision

PostgreSQL COPY crosses the adapter boundary as bounded immutable byte chunks.
COPY OUT is pull-driven and retains only the current bounded chunk. COPY IN
accepts only already-bounded chunks, validates chunk count, per-chunk bytes, and
aggregate bytes before protocol dispatch, then awaits driver backpressure for
every send. Database client stream and sink types remain private.

COPY OUT reports chunk count and total bytes only after protocol completion.
The selected `tokio-postgres` API ends its public stream at `CopyDone` and does
not expose the following command-completion row count, so TableRock does not
invent one. COPY IN reports the server's `CommandComplete` row count.

## Evidence

PostgreSQL 17.10 and 18.4 Testcontainers fixtures prove:

- a 1,000-row CSV export reconstructs exactly from ordered chunks and byte
  offsets within 2,048-chunk, 16,384-byte chunk, and 4,096-byte aggregate caps;
- chunk Debug output contains sizes and offsets, never payload bytes;
- CSV records split across two input chunks import as three rows;
- COPY IN returns exact chunk, byte, and server-confirmed row counts;
- excessive chunk count and COPY OUT aggregate size fail as
  `CopyLimitExceeded` rather than query or protocol errors;
- malformed CSV fails as a redacted query error; and
- a valid COPY IN succeeds after malformed input, proving protocol/session
  recovery.

The official client source confirms COPY IN uses a capacity-one channel and
poll-based sink readiness, while COPY OUT exposes `CopyData` incrementally.

## Safety contract

- Zero limits are rejected before I/O.
- COPY IN validates every bound before creating or truncating the probe table.
- Checked arithmetic prevents chunk-count and aggregate-byte overflow.
- The adapter copies at most one permitted COPY OUT chunk into owned bounded
  memory and never accumulates export data.
- An over-limit COPY OUT stream becomes terminal and never reports completion.
- Payloads, SQL data, and server error detail stay absent from Debug, stable
  errors, and logs.
- Fixed TableRock-owned SQL prevents presentation-supplied execution in this
  feasibility checkpoint.

## Remaining work

This closes PostgreSQL COPY streaming feasibility, not product data transfer.
Open work includes reviewed arbitrary COPY plans, file effects and atomic
destinations, progress/cancellation, partial-file cleanup or retention policy,
format/options validation, encoding, headers, COPY error diagnostics, TLS and
connection-loss matrices, service/runtime ownership, history, TUI/macOS UI,
UniFFI projection, and import/export clean-machine evidence.

## Provenance

External concepts: PostgreSQL COPY protocol and tokio-postgres COPY stream/sink
Public sources: <https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-COPY>
and <https://docs.rs/tokio-postgres/0.7.18/tokio_postgres/struct.Client.html>
TableRock requirements: research 01, 06, 10, 14, 20, 30, 31, 32, 77, 112, 118
Implementation source: TableRock-owned fixed probes, limits, outcomes, and tests
Copied code/assets/text: none
