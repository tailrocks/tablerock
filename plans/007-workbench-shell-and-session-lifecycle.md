# Plan 007: Build the workbench shell — context bar, tabs, status bar, session/context lifecycle

> **Executor instructions**: Follow step by step; verify each step; STOP
> conditions binding. Update `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat d8b113b..HEAD -- crates/tablerock-tui crates/tablerock-cli`
> Requires plans 003, 006 DONE (catalog service; connect flow).

## Status

- **Status**: DONE (evidence 208–210; residual: EngineService event pump,
  schema picker UI, real-server catalog sidebar fixture — land with 009)
- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: plans/003, plans/006
- **Category**: direction (Phase 4 first half)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

Connect currently lands on a stub. The workbench frame — context bar, sidebar
catalog, tab strip, status bar — is the container every engine screen (plans
009, 011, 014, 015) renders into. Spec: `docs/product/workbench.md` (read it
first; it defines the layout, context-bar items, tab semantics, status bar,
and failure truth).

## Current state

- Post-006: `Screen` enum has Connections/editor/stub-workbench; sessions
  live in `SessionRegistry`; catalog service (plan 003) returns
  `CatalogSnapshot` per context; `EngineService::next_update` streams
  operation events (`crates/tablerock-engine/src/service.rs:159`).
- TermRock: `SplitPane`, `Tree`, `Tabs`, `StatusBar` published (survey of rev
  `b7f34da`); `Tabs` already used by the shell (`view.rs:117`).
- Spec anchors (`docs/product/workbench.md`):
  - Context bar always current: connection name, database selector, schema
    selector (PostgreSQL only; hidden where engine has no schemas),
    environment tag (production = persistent warning), safety mode, health.
  - Sidebar: filter field on top; Tables/Views/Functions grouped per schema
    (PG); lazy expansion with explicit loading/stale/error per node; subtree
    refresh; click opens object tab.
  - Tabs: object preview tabs become durable on edit/pin/filter/sort; same
    table in several tabs with independent state; dirty/running markers;
    close with staged changes asks once; strip scrolls; keyboard reachable.
  - Status bar: rows/bytes, elapsed, truncation, operation state text,
    pending-change count, focus hints.
  - Failure truth: disconnect keeps stale results inspectable + marks
    operations disconnected; context switch invalidates dependent
    pages/completions by revision; one failed tab never blocks others.
- TEA constraints: submodels are structure not authority; all revision checks
  in root reducer (`docs/architecture/application-pattern.md`).

## Commands you will need

| Purpose | Command | Expected |
|---|---|---|
| TUI/CLI tests | `cargo test -p tablerock-tui -p tablerock-cli` | pass |
| Engine unit | `cargo test -p tablerock-engine --lib` | pass |
| Real-server (Docker) | `cargo test -p tablerock-engine --test postgres_real` | pass |
| Build/lint | `cargo check --workspace --all-targets && cargo clippy --workspace --all-targets` | exit 0 |

## Scope

**In scope**:
- `crates/tablerock-tui/src/model/workbench.rs` — session view state:
  context (database/schema selection + revision), catalog tree model
  (`CatalogModel` — TableRock-local per termrock-integration.md), tab list
  (id, kind, title, dirty/running, per-tab state slot), status facts.
- `crates/tablerock-tui/src/view/workbench.rs` — `SplitPane` (sidebar/content),
  context-bar line, `Tree` for catalog, `Tabs` strip, `StatusBar`.
- Context switching: database selector effect (PG: new connection context —
  engine decides; CH: request context; Redis: logical DB via isolated state,
  see `docs/product/workbench.md` "Context bar"), schema selector (PG only).
  Each switch bumps a context revision; stale catalog/page completions
  dropped by the root reducer.
- Catalog loading effects: initial load + lazy node expansion + subtree
  refresh via plan 003's `refresh_catalog`; per-node loading/stale/failed
  rendering.
- Tab lifecycle: open-object → preview tab; pin/edit → durable; close-with-
  dirty routes through the single unsaved-change dialog (stub policy — real
  staged changes arrive in plan 013).
- Engine-event pump: a per-session executor task draining
  `EngineService::next_update` into ingress messages (this replaces plan
  005's request/response-only pattern for long operations).
- Health in context bar fed by session state (connected/reconnecting/
  disconnected from plan 006's reconnect machinery).
- Tests + evidence + ledger rows (Object tabs, Responsive layout partially,
  Context switcher) + roadmap notes.

**Out of scope**:
- Grid content, SQL tabs (plans 009/011) — tab content renders placeholder
  facts.
- Session restoration (plan 011 scope), quick switcher (Phase 5+).

## Git workflow

Trunk-only, Conventional Commits, `git commit -s`, push per checkpoint:
frame/layout → catalog wiring → context switching → tab lifecycle.

## Steps

### Step 1: Frame + context bar + status bar

Workbench screen replaces the 006 stub: `SplitPane` layout wide/medium;
narrow = drawer per spec ("catalog becomes a drawer… minimum-size state
replaces overlap" — reuse `LayoutMode` breakpoints, `model.rs:167`).
Context bar renders connection/env/safety/health from session state;
selectors are focusable but static until Step 3. Status bar shows operation
state text + focus hints. Render tests all layouts.

**Verify**: `cargo test -p tablerock-tui` → pass.

### Step 2: Catalog sidebar

`CatalogModel` consumes `CatalogSnapshot` via `CatalogCursor` (stale
snapshots rejected — test); `Tree` renders kinds per engine; lazy expansion
triggers `RefreshCatalog` effects per subtree; filter field filters
preserving ancestor paths (pure model-side filtering of loaded nodes);
loading/stale/failed node states rendered as text+glyph. Real-server test:
PG fixture schema renders tables/views/functions with signatures.

**Verify**: `cargo test -p tablerock-tui`; Docker: `--test postgres_real` → pass.

### Step 3: Context switching

Database/schema selectors open a picker (TermRock `Picker`/`List`); switch
emits effect → engine context change → new context revision → catalog
refresh; in-flight completions carrying the old revision are dropped (root
reducer check + test). Redis: logical DB switch uses isolated per-database
connection state (engine already isolates; assert no shared SELECT race per
`docs/product/redis.md` sidebar rule).

**Verify**: `cargo test -p tablerock-tui -p tablerock-cli` → pass.

### Step 4: Tabs + event pump

Tab model: preview→durable promotion rules; multiple tabs per object with
independent per-tab state slots; close-dirty dialog (single policy point);
strip scrolls; running marker driven by the event pump. Event pump: executor
task per active operation draining `next_update` → typed messages (Started/
Page/Terminal projections); one failed tab's terminal failure never touches
other tabs (test). Disconnect: all live operations render `disconnected`,
stale content stays inspectable (test).

**Verify**: `cargo test -p tablerock-tui -p tablerock-cli` → pass.

### Step 5: Evidence + ledger

Evidence: frame/catalog/context/tabs checkpoints with failure truth
(stale-revision drop counts, disconnect behavior). Ledger rows updated;
ROADMAP Phase 4 notes.

**Verify**: full table green; CI green.

## Test plan

- Reducer: context-revision staleness, tab promotion/close policy, catalog
  cursor rejection, disconnect marking.
- Render: layouts wide/medium/narrow/too-small, per-node states, production
  warning in context bar, status-bar states.
- Integration: PG real-server catalog → sidebar; switch schema → subtree
  refresh.
- Exemplars: plan 005 vertical test, `tests/shell.rs` render helpers.

## Done criteria

- [ ] Workbench frame renders all four regions per spec at all layout modes
- [ ] Catalog lazy-loads with explicit per-node loading/stale/error states
- [ ] Context switch invalidates stale completions by revision (test proves a late old-revision completion is dropped)
- [ ] Same object opens in two tabs with independent state slots (test)
- [ ] Disconnect keeps stale content inspectable; operations marked disconnected (test)
- [ ] clippy green; evidence + ledger updated; `plans/README.md` updated

## STOP conditions

- TermRock `SplitPane`/`Tree` lacks a needed neutral capability — STOP
  (TermRock-first contribution path, own checkpoint).
- Redis logical-DB switching cannot guarantee isolation with the current
  session model — STOP (engine design issue, belongs with plan 002/015).
- Event-pump backpressure forces unbounded buffering in the CLI — STOP
  (bounded channels are an invariant).

## Maintenance notes

- Plans 009/011/014/015 fill tab content; they must not add second policy
  points for close-dirty or context invalidation.
- Reviewer: revision checks live in the root reducer only; no per-tab event
  loops.
