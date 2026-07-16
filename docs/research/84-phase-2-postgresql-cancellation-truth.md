# Phase 2 PostgreSQL Cancellation Truth Evidence

## Checkpoint

The PostgreSQL adapter now distinguishes cancel-request transport acceptance
from cancellation confirmed by the server. The public tracer remains a fixed,
read-only `pg_sleep` probe; arbitrary SQL remains unavailable until the shared
safety classifier and execution contract exist.

This is not the Phase 2 PostgreSQL spike exit. It proves cancellation truth and
post-cancel connection reuse on the pinned PostgreSQL 18.4 fixture. The later
[`136-phase-2-postgresql-tls-identity.md`](136-phase-2-postgresql-tls-identity.md)
checkpoint closes TLS cancellation and PostgreSQL 17.10 coverage; completion
races, connection loss, reconnect, and ambiguous writes remain required.

## Contract decision

`tokio-postgres` documents that a successful cancel call proves only that the
separate cancellation connection delivered its request. PostgreSQL returns no
cancel-request outcome on that connection, and cancellation races with normal
query completion. TableRock therefore reports:

- `ConfirmedByServer` only when the target query ends with SQLSTATE `57014`
  (`query_canceled`);
- `RequestAcceptedButQueryCompleted` when cancel transport succeeds but the
  target query completes normally;
- a message-free `CancellationTransport` failure when the separate cancel
  connection fails;
- the existing message-free query failure for any other target-query error.

The enum prevents presentation code from upgrading request delivery into a
false cancellation claim. No server text, SQL, host, user, database, cell value,
or certificate path crosses the adapter boundary.

## Real-server evidence

Testcontainers Rust 0.27.3 starts the official `postgres:18.4-alpine` image on
an ephemeral host port. The test starts a 30-second read-only sleep, waits 150
milliseconds so it is in flight, sends a protocol cancellation request, and
requires SQLSTATE `57014`. It then streams a fresh bounded query on the same
session, proving the connection remains usable, and shuts the session down.

The disabled-TLS fixture uses `NoTls`. The obsolete downgrade-capable `Prefer`
mode has since been removed. Required TLS cancellation now retains and clones
the exact connection connector, including custom roots, server name, and client
identity; both supported-line real fixtures pass in checkpoint 136.

## Verification record

- Engine unit tests: pass.
- PostgreSQL 18.4 Testcontainers streaming and cancellation tests: pass.
- Full workspace, lint, documentation, dependency, secret, and English gates:
  recorded in the publishing commit.

External concepts: PostgreSQL cancellation request protocol, SQLSTATE cancellation confirmation, cancellation race semantics
Public sources: <https://docs.rs/tokio-postgres/0.7.18/tokio_postgres/struct.CancelToken.html>, <https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-CANCELING-REQUESTS>, <https://www.postgresql.org/docs/current/errcodes-appendix.html>
Implementation source: TableRock-owned adapter and independent Testcontainers fixture
Copied code/assets/text: none
