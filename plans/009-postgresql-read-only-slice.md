# Plan 009: PostgreSQL read-only vertical slice — data grid, SQL streaming, inspector (Phase 4 exit)

> **Executor instructions**: Follow step by step; verify each step; STOP
> conditions binding. Update `plans/README.md` when done. Read
> `docs/product/data-grid.md` and `docs/product/workbench.md` before coding —
> they are the behavioral authority.
>
> **Drift check (run first)**: `git diff --stat d8b113b..HEAD -- crates/tablerock-tui crates/tablerock-cli crates/tablerock-engine`
> Requires plans 002, 003, 007, 008 DONE.

## Status

- **Priority**: P1
- **Effort**: L
- **Risk**: MED
- **Depends on**: plans/002, plans/003, plans/007, plans/008
- **Category**: direction (Phase 4 roadmap exit)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

Phase 4 is the first full read path: click a table → bounded pages in a real
grid; run arbitrary SQL → streaming typed results with progress, cancel, and
inspection. It is the template every other engine slice copies. Exit evidence
(delivery-plan.md Phase 4): first rows render before completion; stale pages
cannot cross reconnect/context/query revisions; resident scrolling performs
no I/O; caps exact; unknown values inspectable but non-editable; cancellation
reports observed race outcome.

## Current state

- Available after deps: arbitrary-statement execution + persistent sessions
  (plan 002: `DriverPageRequest::PostgreSqlStatement`,
  `CommandIntent::Execute`), catalog sidebar + tabs + event pump (007),
  TermRock `VirtualGrid` (008).
- Engine page delivery: `EngineServiceUpdate::{Started, Page(Box<ResultPage>), CancelDispatched, Terminal}`
  (`crates/tablerock-engine/src/service.rs:60-66`); pages admitted to the
  bounded `ResultStore` (`tablerock-core/src/result_store.rs:148` —
  `open_result/admit/get/set_pinned`, LRU, revision-gated).
- `ResultPage::cell(row, col) -> CellRef` with kind/null/truncation/bytes
  (`tablerock-core/src/page.rs:574,822`); `ValueKind` distinguishes
  Null/Boolean/Signed/Unsigned/Float64/Decimal/Temporal/Text/Structured/
  Binary/Invalid/Unknown (`value.rs:467`).
- PostgreSQL decoders cover the full researched family matrix (evidence docs
  167–191): JSON, arrays, ranges, composites, domains, enums, network, bit
  strings, identifiers, LSN, TID, OID vectors, snapshots, numeric, UUID,
  temporal.
- Budgets (fixed decision "Result budgets and encoding"): 500-row pages,
  10,000-row arbitrary-query cap, 2 MiB page residency, no unlimited mode.
- Spec anchors (`docs/product/data-grid.md`): bounded server pages
  (default 500); totals exact/estimated/unknown labeled; typed distinctions
  visible never color-alone; inspector with text/JSON/hex + metadata + stale
  state; operation states idle/queued/running/streaming/completed/
  cancel-requested/cancelled/failed/disconnected with elapsed + rows/bytes +
  cancel while running; failed loads keep stale pages visible.
- Sorting/filtering/column controls are Phase 5 (plan 012) — NOT here. The
  grid ships browse+inspect only.

## Commands you will need

| Purpose | Command | Expected |
|---|---|---|
| TUI/CLI tests | `cargo test -p tablerock-tui -p tablerock-cli` | pass |
| Engine (Docker) | `cargo test -p tablerock-engine --test postgres_real` | pass |
| Perf (Docker, local only) | `cargo test -p tablerock-engine --test performance_real` | pass |
| Build/lint | `cargo check --workspace --all-targets && cargo clippy --workspace --all-targets` | exit 0 |

## Scope

**In scope**:
- `crates/tablerock-tui/src/model/grid.rs` — `DataGridModel`
  (TableRock-local by decision): resident page window over projected cells,
  row/column identity, viewport state bridging to `VirtualGridState`,
  operation-state machine projection, totals/truncation facts, per-tab
  ownership.
- `crates/tablerock-tui/src/view/grid.rs` — `VirtualGrid` composition:
  typed-distinction rendering (NULL/empty/binary/structured/truncated/
  unknown as distinct glyph+text treatments), pending-cell placeholders,
  status-bar facts.
- `crates/tablerock-tui/src/model/inspector.rs` + `view/inspector.rs` —
  full-value inspector panel: text/JSON/hex projections, metadata
  (engine type, kind, truncation, byte length), stale marker.
- Table browsing effect: open table tab → `Execute` a bounded
  `SELECT * FROM <qualified> LIMIT/OFFSET`-free page plan — **use the
  bounded statement path with explicit page fetches**: browse issues
  `SELECT ... FROM ONLY? <table>` via keyset-free bounded streaming, pages
  admitted to `ResultStore`, `FetchPage` requests pull further pages on
  scroll-past-resident. Identifier qualification uses parameterized
  identifier quoting built engine-side (never string-concatenated user text
  — build a `quote_ident` helper in the engine with tests).
- SQL tab (minimal): a single-line statement input (TermRock `TextInput`;
  the real editor is plan 011) + run/cancel + the same grid for results +
  redacted error line with position when available.
- Cancel: reachable while running; renders dispatch + observed outcome
  distinctly (`CancelDispatch` vs terminal outcome).
- Scroll-driven page fetches with resident-window eviction via `ResultStore`
  (pin the viewport page, `set_pinned`).
- Tests + evidence + ledger rows (Paging, Typed display, Row/value
  inspector, partial Virtualized grid) + ROADMAP Phase 4 exit.

**Out of scope**:
- Sorting/filtering/column controls (plan 012), editing (plan 013),
  multiline editor/completion/history (plan 011), copy formats (plan 012),
  structure/DDL tab (goes here ONLY as raw definition text via
  `pg_get_viewdef`/`pg_catalog` if trivial — otherwise defer to plan 013;
  decide by effort, note in evidence).
- ClickHouse/Redis grids (plans 014/015 reuse the model).

## Git workflow

Trunk-only, Conventional Commits, `git commit -s`, push per checkpoint:
grid model → table browsing → SQL tab + cancel → inspector → evidence.

## Steps

### Step 1: `DataGridModel` + grid view over `VirtualGrid`

Model owns: result identity (result_id + revision), resident window
(Vec of projected page slices), totals, operation state, viewport. Cell
projection: `CellRef` → display string + distinction class, computed OUTSIDE
render (precomputed per admitted page — TEA: no per-frame decoding).
Render tests: every `ValueKind` renders distinctly (fixture page via
`ResultPage::from_row_major`), truncation marker, pending placeholder,
too-narrow clipping.

**Verify**: `cargo test -p tablerock-tui` → pass.

### Step 2: Table browsing end-to-end

Open table from catalog → effect: open result in `ResultStore`, submit
bounded browse statement, pump pages → grid fills; scroll past resident →
`FetchPage` effect → next page admitted (LRU evicts distant pages; pinned
viewport page never evicted — test). Status bar: rows/bytes/elapsed/
truncation/state. Real-server test: 2,500-row fixture table browses in 500-row
pages; scrolling fetches; first page renders before stream completion
(assert Started+Page precede Terminal in message order).

**Verify (Docker)**: `cargo test -p tablerock-engine --test postgres_real` +
`cargo test -p tablerock-cli` → pass.

### Step 3: SQL tab + cancel + errors

Single-line input → Execute → same grid path. Cancel button/key while
running → `CancelDispatched` rendered ("cancel requested"), terminal
outcome rendered honestly (`ServerConfirmedCancelled` vs
`CompletedBeforeCancel` vs `ClientStopped` — distinct labels; evidence 155
semantics). Error: redacted class + position mapped when the diagnostic
carries one. Real-server test: `pg_sleep` query cancelled → observed outcome
label correct; syntax error → session usable after.

**Verify (Docker)**: as Step 2.

### Step 4: Inspector

Panel opens on selected cell: full value text (from the owned value —
refetch the single value if truncated? NO: truncated stays truncated with
its original-length fact; refetch is a later capability — render the
truncation honestly), JSON tree for `Structured`, hex for `Binary`,
metadata rows via `DetailTable`. Stale marker when the owning result's
revision is superseded.

**Verify**: `cargo test -p tablerock-tui` → pass.

### Step 5: Phase 4 exit evidence

Prove each delivery-plan exit item with a named test: first-rows-before-
completion; stale-page rejection across context/query revision (reuse plan
007's revision-drop machinery); resident scroll does no I/O (assert no
FetchPage effect within resident window); caps exact (10,000-row cap
truncation labeled); unknown values inspectable, non-editable (editing
doesn't exist yet — assert no edit affordance); cancellation race truth.
Ledger + ROADMAP updates.

**Verify**: full table green; CI green.

## Test plan

- Model: page admission windows, pinning, eviction, operation-state machine,
  distinction classes per `ValueKind` (12 kinds — table-driven test).
- Render: `TestBackend` fixtures incl. Unicode-wide cells, narrow layouts.
- Real-server: browse paging, cancel races, error recovery, wide-type fixture
  (reuse the typed-values fixture from `postgres_real.rs`
  `streams_typed_values_from_supported_postgres_lines`).
- Perf: run `performance_real` locally; budgets unchanged.

## Done criteria

- [ ] Table browse: 500-row pages, scroll-fetch, pinned viewport, LRU eviction (tests)
- [ ] All 12 `ValueKind`s render distinctly; never color-alone (test asserts glyph/text differs)
- [ ] First rows render before completion (test)
- [ ] Stale revisions cannot deliver (test)
- [ ] Cancel renders requested vs observed outcome distinctly (real-server test)
- [ ] Inspector: text/JSON/hex + metadata + stale (tests)
- [ ] clippy green; evidence + ledger + ROADMAP Phase 4 updated; `plans/README.md` updated

## STOP conditions

- `ResultStore` API can't express the viewport-pinning pattern (check
  `set_pinned` + `PinnedCapacity` semantics first) — STOP.
- Browse requires OFFSET-style paging (unbounded server cost) because the
  streaming path can't suspend/resume — re-read plan 002's stream design;
  if pages can only arrive by continuous pump, adapt the model to
  pump-and-store (store all ≤10,000-cap rows via ResultStore admission,
  which the budgets permit) and record the decision; if neither fits, STOP.
- `VirtualGrid` API (plan 008) misses a needed capability — STOP; upstream
  fix first.

## Maintenance notes

- Plans 012 (sort/filter/columns/copy), 013 (editing) extend
  `DataGridModel`; keep projection/precompute separate from policy so those
  plans add, not rewrite.
- Reviewer: no decoding in view code; cancel labels never claim server
  cancellation without proof.
