# Workbench catalog tree and context-revision gating

Date: 2026-07-18

## Checkpoint

Plan 007 steps 2–3 (partial). Workbench loads catalog roots after Connect
via `Effect::LoadCatalog` against the registered session. Lazy expand
requests child levels. Context database switch bumps `context_revision`
and reloads; stale catalog completions are rejected by the root reducer.

## Decision

- Presentation-local `CatalogModel` / `CatalogNodeProjection` (no engine
  types in TUI). Filter preserves ancestors; collapsed branches hide
  descendants.
- Executor maps `CatalogLevelSpec` → engine `CatalogRequest` and projects
  seeds into node rows with path ids (`db`, `db/schema`, …).
- `accepts(token, context_revision)` gates CatalogLoaded/Failed.
- `NextDatabase` cycles root databases from the loaded catalog and reloads
  at the new revision (engine connection-context change deferred; UI
  revision isolation is proven first).

## Evidence

- `model::catalog::tests::*` (filter ancestors, collapse, accepts)
- `update::tests::catalog_loaded_merges_roots_and_rejects_stale_revision`
- `update::tests::database_switch_bumps_revision_and_reloads_catalog`
- `update::tests::connect_opens_workbench_and_disconnect_returns` (LoadCatalog)
- Log: implementer `catalog-context-tests.log`

## Remaining work

- True engine database/schema context switch (PG reconnect / CH request /
  Redis logical DB isolation).
- Tab lifecycle + EngineService event pump (step 4).
- Real-server catalog sidebar fixture assertion.
