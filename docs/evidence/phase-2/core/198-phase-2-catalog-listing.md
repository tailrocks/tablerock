# Phase 2 catalog listing service

Date: 2026-07-18

## Checkpoint

Plan 003. All three engines list lazy catalog levels behind
`DriverSession::catalog` and `EngineService::refresh_catalog`.

## Decision

- `CatalogRequest` / `CatalogNodeSeed` / `CatalogSubtree` / `CatalogExactness`
  in `tablerock-engine` (identifiers redacted in Debug).
- PostgreSQL: `pg_database` / `pg_namespace` / `pg_class`+`pg_proc` with
  `pg_get_function_arguments` for function signatures (stored as
  `EngineType` text).
- ClickHouse: `system.databases` / `system.tables` / `system.dictionaries`
  via TabSeparated string lists.
- Redis: `CONFIG GET databases` → `db0..dbN-1`; ACL/CONFIG denial uses
  default 16 with `CatalogExactness::DefaultAssumed`.
- `EngineService::refresh_catalog` requires `RefreshCatalog` intent, calls
  session catalog, advances context scope revision, assembles
  `CatalogSnapshot`, accepts `CatalogCursor` only for exact next revision.

## Bounds and failure truth

- Limit +1 fetch detects truncation → `CatalogExactness::Truncated` /
  `complete: false`.
- Hostile PG name `semi;--x` listed verbatim (parameterized schema filter).
- Catalog failures leave scope revision unchanged when listing fails before
  advance; advance happens only after successful subtree return.

## Evidence

- `cargo test -p tablerock-engine --lib --test engine_service`
- `cargo test -p tablerock-engine --test postgres_real lists_catalog`
- `cargo test -p tablerock-engine --test clickhouse_real lists_catalog`
- `cargo test -p tablerock-engine --test redis_real lists_catalog`

## Remaining work

- UI tree projection (plan 009).
- Redis namespace grouping (plan 015).
- Column-level expansion (later structure views).
