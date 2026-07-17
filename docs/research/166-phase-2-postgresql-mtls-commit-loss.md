# Phase 2 PostgreSQL mTLS Commit-Loss Evidence

Date: 2026-07-17

## Decision

Required custom-root TLS and client identity do not weaken PostgreSQL ambiguous
commit truth. Transport loss during active COMMIT remains
`WriteOutcomeUnknown`; recovery requires a freshly resolved endpoint and the
same verified TLS policy. Plaintext fallback is forbidden.

This composes the research 136 TLS identity contract with the research 165
activity-gated COMMIT-loss tracer. No second TLS, cancellation, or mutation path
is introduced.

## Evidence

PostgreSQL 17.10 and 18.4 Testcontainers fixtures require:

- two custom-root, hostname-verified, client-certificate sessions;
- activity proof that deferred COMMIT is executing before server stop;
- unknown write outcome and terminal old TLS sessions after transport loss;
- same-data-directory restart with refreshed random host-port mapping;
- explicit rejection of plaintext recovery;
- bounded recovery using the original custom CA, server-name override, client
  certificate, and private key;
- zero durable rows after PostgreSQL aborts the in-progress transaction; and
- no automatic transaction replay.

## Safety contract

- Recovery cannot downgrade `Required` TLS to plaintext.
- Trust roots, server identity, and client identity are revalidated on the new
  connection.
- Credentials and certificate/key material remain borrowed, bounded, and
  absent from Debug/errors/default logs.
- Terminal sessions cannot reconnect themselves or replay transaction intent.
- Activity gating and recovery attempts remain bounded.
- Later rollback observation never rewrites the original unknown outcome.

## Remaining work

This closes required-mTLS composition for active-COMMIT transport loss.
Connection loss before dispatch, during request transmission, and after durable
commit but before response; TLS identity rotation during recovery; shared-service
reconnect ownership; reviewed plans; conflicts; history/UI/UniFFI remain open.

## Provenance

External concepts: PostgreSQL TLS/client certificates, deferred COMMIT,
shutdown/restart, and Testcontainers lifecycle
Public sources: <https://www.postgresql.org/docs/current/ssl-tcp.html>,
<https://www.postgresql.org/docs/current/sql-createtrigger.html>, and
<https://docs.rs/testcontainers/0.27.3/testcontainers/core/struct.ContainerAsync.html>
TableRock requirements: research 01, 06, 10, 13, 14, 20, 30, 31, 32, 136, 165
Implementation source: TableRock-owned TLS fixture and commit-loss probes/tests
Copied code/assets/text: none
