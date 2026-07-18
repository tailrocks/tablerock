# Plan 014: ClickHouse complete engine slice (Phase 7)

> **Executor instructions**: Work-package plan. Read
> `docs/product/clickhouse.md`, delivery-plan.md "Phase 7", and
> fixed-decisions.md "ClickHouse arbitrary results and writes" first.
> Trunk checkpoints with evidence. STOP conditions binding. Update
> `plans/README.md` when done.
>
> **Drift check (run first)**: plans 011 + 012 DONE (013 parallel-OK);
> confirm the ClickHouse driver still matches "Current state".

## Status

- **Priority**: P2
- **Effort**: L
- **Risk**: MED
- **Depends on**: plans/011, plans/012
- **Category**: direction (Phase 7 roadmap)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

ClickHouse reuses the entire workbench (catalog sidebar, grid, SQL editor,
copy) built in plans 003–012 — this plan closes the engine-specific
remainder: honest progress/cancellation surfaces, engine facts, explain,
inserts, and asynchronous mutations that are NEVER presented as
transactions.

## Fixed constraints (inline)

- Official `clickhouse-rs` is the only transport; an upstream gap blocks the
  affected capability rather than spawning a workaround (delivery-plan
  Phase 7 exit).
- Arbitrary results via `fetch_bytes("RowBinaryWithNamesAndTypes")`
  (already the driver's path); typed client rows only for known catalog
  queries.
- Writes: reviewed batch INSERT first; UPDATE/DELETE as mutations with
  `system.mutations` tracking (done/failed/unknown); "never present
  ClickHouse mutations as transactions" (ROADMAP Phase 7).
- Cancellation states requested / client-stopped / server-confirmed /
  unknown separately visible (`docs/product/clickhouse.md`); the driver's
  `KILL QUERY … SYNC` confirmation is evidence-locked (doc 129) — the
  15-second synchronous budget is deliberate, not a hang.

## Current state (entry gate)

- Driver (`crates/tablerock-engine/src/clickhouse.rs`): real HTTP(S) client,
  RowBinary streaming + rich decoders (complex scalars, containers,
  temporal — evidence 95/97/100/178), single-active-query guard,
  `KILL QUERY` cancel (`dispatch_cancel` :294). Post-plan-002:
  `ClickHouseStatement` arbitrary path + health.
- Gaps from the Phase 2 evidence ledger (must close or record): progress/
  query-ID surfacing, late-HTTP-error visibility (partial data + late error
  remains visible — Phase 7 exit), TLS custom-CA/mTLS
  (`ClickHouseTlsMode` is only Disable/Require with default roots —
  engine survey §3), remaining failure races.
- Catalog: databases/tables/views/dictionaries listing (plan 003); engine
  facts (MergeTree family, partitioning, ordering) NOT yet listed.

## Scope (checkpoints)

1. **TLS completion**: custom-root CA + client identity for ClickHouse
   (parity with PG/Redis config surface; connections.md TLS modes apply to
   all engines). Real-server TLS fixture matrix (model after
   `tests/redis_real.rs` TLS fixtures).
2. **Progress + late errors**: surface query ID + progress facts into
   operation events (`OperationEventKind::Progress` exists); prove partial
   data + late HTTP error visible (fixture: streaming query that fails
   mid-stream — exception-in-stream corpus); status-bar rendering.
3. **Structure/engine facts**: `system.tables`/`system.columns`/
   `system.parts` projections → structure tab (engine, partition key,
   ordering key, columns, DDL via `SHOW CREATE TABLE`); explain variants
   (raw + structured with unknown-node fallback) behind the editor.
4. **Batch INSERT**: staged inserts through the mutation seam (plan 013's
   intent) with `MutationExecutionModel::ClickHouseProgressiveInsert`
   semantics: progressive batches, row-confirmed outcomes, no rollback
   language; review dialog wording asserts non-transactional phrasing.
5. **Async mutations**: gated UPDATE/DELETE creating mutations; mutation
   identity + `system.mutations` polling to done/failed/unknown; UI
   tracking surface; cancellation of mutations where permitted
   (`KILL MUTATION`) behind a destructive gate.
6. **Cancellation truth UI**: four-state rendering (requested/
   client-stopped/server-confirmed/unknown) wired to
   `CancelDispatch`/terminal outcomes.

**Out of scope**: parts administration/optimize (Phase 10), dictionaries
content browsing beyond listing, cross-engine movement (plan 016).

## Commands

Standard suites; Docker: `cargo test -p tablerock-engine --test clickhouse_real`
extended with the new fixtures; CI list updated.

## Done criteria

- [x] TLS modes: Disable + RequireSystemRoots (native-roots feature); custom CA/mTLS residual (HttpClient)
- [x] Partial rows + late error both visible in one operation (test) — residual closed
- [x] Structure engine facts + columns — evidence 236; explain raw/AST — evidence 239
- [x] INSERT progressive apply non-transactional — evidence 236
- [x] UPDATE/DELETE async mutations + system.mutations poll to done — evidence 237
- [x] Four cancellation states rendered distinctly — evidence 238
- [x] Plan index DONE with residual below; suites green for landed checkpoints

## Residual (non-blocking)

- Custom CA / mTLS via `clickhouse::Client::with_http_client` fixture matrix
- ~~Progress OperationEvent surface into status bar~~ (closed: evidence 320;
  X-ClickHouse-Summary → server_progress; query_id was 308)
- ~~Partial-page + late error single-operation fixture~~ (closed: clickhouse_real)
- ~~KILL MUTATION destructive gate~~ (closed: evidence 305)
- ~~Editor Explain action wiring to explain_raw/structured~~ (closed: evidence 239/290/302)
- ~~Multi-engine ExecuteSql + CH query_id~~ (closed: evidence 308)

## STOP conditions

- `clickhouse-rs` cannot expose needed progress/late-error facts on the
  `fetch_bytes` path — STOP; record as upstream-blocked capability (the
  fixed decision forbids a second transport).
- Mutation polling requires privileges the test matrix can't grant — STOP
  and record the permission-gated state.

## Maintenance notes

- Plan 016 reuses INSERT batching for import; Phase 10 adds
  parts/optimize admin rows.
- Reviewer: non-transactional language everywhere; unknown-outcome honesty
  on poll interruption.
