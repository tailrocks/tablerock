# Phase 2 PostgreSQL Ambiguous Commit Evidence

Date: 2026-07-17

## Decision

A transaction whose `COMMIT` was dispatched but whose completion was not
observed is `WriteOutcomeUnknown`. TableRock does not infer rollback, infer
commit, or replay the transaction.

The fixed tracer uses an `AFTER INSERT` constraint trigger declared `DEFERRABLE
INITIALLY DEFERRED`. PostgreSQL therefore executes its one-second delay at
transaction end. Client response observation ends after 200 ms while COMMIT is
in progress, without sending a cancel request.

## Evidence

PostgreSQL 17.10 and 18.4 Testcontainers fixtures prove:

- the transaction response exits as `PostgresError::WriteOutcomeUnknown`;
- an independent session later observes exactly one committed row;
- the original session drains the abandoned completion and observes the same
  one-row state; and
- no automatic transaction replay produces a second identity row.

This is distinct from research 163: that tracer delays the write statement;
this tracer delays only a deferred trigger fired at transaction completion.

## Safety contract

- Schema and deferred trigger setup complete before the timed transaction.
- The INSERT is inside an explicit `BEGIN`/`COMMIT` batch.
- Only an explicit cancel operation can claim or request cancellation.
- Unknown remains unknown even if later observation establishes durable state.
- Recovery drains protocol state but never replays transaction text.
- Stable errors and default logs contain no SQL text or values.
- Fixed TableRock-owned SQL prevents presentation-supplied execution.

## Remaining work

This closes applied-after-unobserved-COMMIT feasibility. Research 165
subsequently closes rollback observation after transport loss during COMMIT.
Connection loss before dispatch, mid-request, and after server commit but before
response; TLS loss; reconnect ownership; reviewed transactional mutation
plans; conflict handling; service/history/UI/UniFFI projection remain open.

## Provenance

External concepts: PostgreSQL deferred constraint-trigger timing, transaction
completion, and tokio-postgres batch request ownership
Public sources: <https://www.postgresql.org/docs/current/sql-createtrigger.html>,
<https://www.postgresql.org/docs/current/protocol-flow.html>, and
<https://docs.rs/tokio-postgres/0.7.18/tokio_postgres/struct.Client.html>
TableRock requirements: research 01, 06, 10, 13, 14, 20, 30, 31, 32, 163
Implementation source: TableRock-owned deferred-trigger transaction probe/tests
Copied code/assets/text: none
