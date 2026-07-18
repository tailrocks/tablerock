# Phase 2 arbitrary statements and session health

Date: 2026-07-18

## Checkpoint

Plan 002 steps 3–4. PostgreSQL and ClickHouse accept operator-supplied
statements through the shared page-stream path; all three engines expose
`health()`; one registered session survives multi-operation reuse.

## Decision

- `DriverPageRequest::PostgreSqlStatement` / `ClickHouseStatement` carry
  `StatementText` (Debug length-only) plus the same page limits as probes.
- Streaming reuses existing decoder/page assembly (`stream_statement` on each
  session). Probe enums remain for evidence fixtures.
- `DriverSession::health` returns `SessionHealth { engine, server_reachable,
  elapsed_millis }` via cheap round-trips: PG `SELECT 1`, CH `SELECT 1`
  RowBinary, Redis `PING`. No version strings in this checkpoint.
- Multi-operation proof: register one session, run statement → statement →
  cancel → health → syntax-error → health; then exclusive disconnect.

## Bounds and failure truth

- Empty statement rejected pre-I/O (`InvalidLimits`).
- Syntax / query errors map to `AdapterFailureClass::Query` without closing the
  registered session.
- SQL text never appears in `Debug` of requests or `StatementText`.
- ClickHouse readiness: container start alone is not enough; health-poll before
  claims.

## Evidence

- `cargo test -p tablerock-engine --lib --test engine_service --test driver_runtime --test session_registry`
- `cargo test -p tablerock-engine --test postgres_real persistent_session`
- `cargo test -p tablerock-engine --test clickhouse_real persistent_session`

## Remaining work

- Catalog listing (plan 003).
- TUI effect bridge (plan 005).
- Redis command editor (plan 015).
