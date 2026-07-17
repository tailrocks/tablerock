# Phase 2 PostgreSQL Commit Transport-Loss Evidence

Date: 2026-07-17

## Decision

Transport loss while PostgreSQL is executing COMMIT yields
`WriteOutcomeUnknown`. The old session is terminal. Recovery requires an
explicit new connection using freshly resolved endpoint facts and never replays
the transaction.

The tracer waits until `pg_stat_activity` proves the transaction backend is
inside a ten-second deferred commit trigger, then stops the server with a
one-second Docker grace window. This avoids timing guesses: loss occurs only
after COMMIT is actively executing on the server.

## Evidence

PostgreSQL 17.10 and 18.4 same-container Testcontainers fixtures prove:

- setup is durable before the tested transaction;
- an independent observer sees the transaction backend active in `PgSleep`;
- server stop makes the dispatched transaction `WriteOutcomeUnknown`;
- both old sessions terminate as `Connection` failures;
- the same container/data directory restarts;
- recovery re-reads the current random host-port mapping and creates a new
  session under bounded per-attempt and aggregate readiness limits;
- PostgreSQL shutdown aborts the in-progress transaction, so recovery observes
  zero rows; and
- no automatic replay changes that zero-row state.

Research 164 proves the complementary result: an unobserved COMMIT can durably
apply exactly once. Together, durable application and rollback are both valid
behind the same unknown outcome.

## Safety contract

- Server activity, not elapsed time, gates transport termination.
- Any error after transaction dispatch remains unknown.
- Terminal sessions cannot be reused or silently replaced.
- Reconnect refreshes endpoint mapping and has a two-second per-attempt cap
  inside a 30-second aggregate bound.
- Later observation does not retroactively rewrite the original outcome.
- Recovery never replays SQL or mutation intent.
- Stable errors/logs contain no SQL text, identifiers, or values.

## Remaining work

Connection loss before dispatch and during request transmission, loss after
durable commit but before response, TLS loss, explicit reconnect ownership in
the shared service, reviewed plans, history/UI/UniFFI projection, and general
transaction conflicts remain open.

## Provenance

External concepts: PostgreSQL deferred triggers, fast shutdown, transaction
rollback, activity visibility, and Testcontainers same-container restart
Public sources: <https://www.postgresql.org/docs/current/sql-createtrigger.html>,
<https://www.postgresql.org/docs/current/server-shutdown.html>, and
<https://docs.rs/testcontainers/0.27.3/testcontainers/core/struct.ContainerAsync.html>
TableRock requirements: research 01, 06, 10, 13, 14, 20, 30, 31, 32, 164
Implementation source: TableRock-owned activity-gated loss/recovery probes/tests
Copied code/assets/text: none
