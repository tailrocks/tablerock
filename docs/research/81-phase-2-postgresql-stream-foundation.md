# Phase 2 PostgreSQL Stream Foundation Evidence

## Checkpoint

The new `tablerock-engine` crate owns the first PostgreSQL adapter boundary.
Exact `tokio-postgres` 0.7.18 and `tokio-postgres-rustls` 0.14.0 driver types stay
private. The public seam contains only TableRock configuration facts, a fixed
read-only feasibility probe, and bounded immutable core pages.

This historical checkpoint was superseded by the sole extended-query typed path
in [`87-phase-2-postgresql-typed-stream.md`](87-phase-2-postgresql-typed-stream.md).
It proved driven connection ownership and bounded text-protocol streaming
against one pinned real server;
typed binary values, parameters, TLS fixtures, authentication, notices, COPY,
cancel races, reconnect, connection loss, and ambiguous writes remain required.

## Dependency decision

`tokio-postgres` uses its default `runtime` feature. Disabled-TLS fixtures use
the driver's `NoTls` transport and never depend on a certificate store. TLS
profiles use the rustls adapter with only `ring` and `native-certs`; platform
roots are loaded without exposing certificate errors or paths. The first
candidate used `webpki-roots`, but the
license gate rejected its unapproved CDLA-Permissive-2.0 data license. It was
removed rather than adding an exception. The accepted graph passes configured
license, ban, and source policy. Both selected crates satisfy the workspace
Rust 1.95 baseline (`tokio-postgres` declares MSRV 1.85).

Context7 was attempted first for Testcontainers API guidance and reported its
monthly quota exhausted. Version, features, lifecycle, readiness, mapped-port,
and Drop-cleanup behavior were then verified from Cargo metadata and the
official Testcontainers Rust 0.27.3 documentation/source.

## Ownership, bounds, and safety

- `PostgresSession` owns the client and a continuously driven connection task.
  Explicit shutdown drops the client and joins that task.
- `PostgresTextStream` privately owns the pinned driver stream. It emits at most
  the configured page rows, arena bytes, column count, column metadata bytes,
  and per-cell bytes.
- UTF-8 truncation backs up to a scalar boundary, retains original byte length,
  and sets `ByteLimitReached`. Page lookahead retains the first row of the next
  page and sets `RowLimitReached`; no row is lost or duplicated.
- Driver/server errors collapse to message-free metadata categories. Config
  Debug exposes lengths, port, and TLS mode only.
- No raw SQL API is public. Until the Rust safety classifier and execution
  contract exist, the adapter exposes only `PostgresProbeQuery::BoundedSeries`.
  This structurally prevents an early write-policy bypass.
- The simple-query text feasibility path described here has been deleted. The
  current adapter uses only the extended-query typed path. User parameters are
  never interpolated.

## Real-server evidence and support matrix

| Server | Fixture | Evidence | Support claim |
|---|---|---|---|
| PostgreSQL 18.4 | Testcontainers Rust 0.27.3 starts official `postgres:18.4-alpine` with an ephemeral mapped port and trust auth confined to the disposable fixture | driven connect/shutdown; three rows streamed as 2+1 pages; NULL; Unicode scalar-safe truncation; row/byte warnings; stable ordering | tracer only |
| PostgreSQL 17.10 | required preceding production line | not yet run | none |

The Testcontainers handle owns lifecycle and removes the 18.4 fixture on Drop.
Trust authentication and disabled TLS are fixture-only; neither is a public
support claim.

## Verification record

- Engine unit tests: 2 passed.
- Pinned PostgreSQL 18.4 Testcontainers real-server test: 1 passed.
- `cargo test --workspace --all-targets --locked`: 110 passed, 3 ignored.
- `cargo deny check licenses bans sources`: pass after removing
  `webpki-roots`.
- `gitleaks detect --redact`: pass.
- Full workspace verification is recorded in the publishing commit.

External concepts: asynchronously driven PostgreSQL connections, simple-query result framing, platform-root rustls, bounded incremental pages
Public sources: <https://docs.rs/tokio-postgres/0.7.18>, <https://docs.rs/tokio-postgres-rustls/0.14.0>, <https://docs.rs/testcontainers/0.27.3>, <https://www.postgresql.org/docs/current/protocol-flow.html>, <https://www.postgresql.org/support/versioning/>
Implementation source: TableRock-owned adapter, core page contract, and independent fixtures
Copied code/assets/text: none
