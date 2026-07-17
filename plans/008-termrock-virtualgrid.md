# Plan 008: Contribute `VirtualGrid` to TermRock (T2) and pin it from TableRock

> **Executor instructions**: This plan spans TWO repositories: the TermRock
> repo (github.com/tailrocks/termrock, clone as a sibling checkout) and this
> one (only for the final pin bump). Follow the TermRock contribution gate
> exactly. STOP conditions binding. Update `plans/README.md` when done.
>
> **Drift check (run first)**: in TableRock,
> `grep -n "termrock" Cargo.toml` — pinned rev must still be
> `b7f34da8db5842bb439296fe4cde534de0c1eb3c` (Cargo.toml:24); if TermRock
> `main` moved past it, survey `crates/termrock/src/widgets/` for an existing
> grid before writing any code; a grid already existing = STOP (re-scope to
> adoption only).

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED (cross-repo, API design longevity)
- **Depends on**: none (parallel to 003–007; blocks 009)
- **Category**: direction (TermRock checkpoint T2)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

The data grid is the heart of the product and TermRock has no grid widget:
at pinned rev `b7f34da` the widget set includes `table.rs`/`detail_table.rs`
but no virtualized two-axis grid. The delivery sequence
(`docs/architecture/termrock-integration.md` "Delivery sequence") defines T2:
"`VirtualGrid` with benchmark and lookbook corpus — unblocks
Browsing/results". Fixed decision: missing neutral primitives land on
TermRock `main` first; TableRock never builds a local generic grid.

## Current state

- TableRock pins termrock `=0.11.0` at rev `b7f34da…` with `crossterm`+`serde`
  features (`Cargo.toml:24`).
- Required contract, verbatim from
  `docs/architecture/termrock-integration.md` "Missing neutral primitives"
  table, row `VirtualGrid`: "Borrowed visible cells; stable row/column IDs;
  header/gutter; two-axis viewport; range selection; column widths; hit
  regions; caller render projection; no fetching/edit policy."
  Companion row "Grid/tree scrollbars": "Visible range/unknown total;
  horizontal and vertical state; drag/page behavior; painted-geometry hit
  regions."
- Contribution gate (same doc, "TermRock contribution gate") — all nine
  requirements apply: neutral naming/borrowed data/stable IDs; behavior tests
  (keyboard, mouse, focus, disabled, empty, clipping, minimum rects, Unicode
  display columns); lookbook story + deterministic preview; caller-owned
  policy docs; no Tokio/database deps; performance budget for hot paths;
  Jackin compatibility verification on API change; DCO commit direct to
  TermRock `main`; exact-revision TableRock pin bump afterward.
- TableRock consumption target (for API-fit validation only, not to build
  now): `DataGridModel` composes typed values/pages/editability locally
  (termrock-integration.md "Deliberately TableRock-local").
- Parity ledger row "Virtualized grid": "Render only resident rows/columns;
  stable two-axis navigation and placeholders; no I/O from render |
  Million-row synthetic viewport benchmark".

## Commands you will need

| Purpose | Command (in termrock checkout) | Expected |
|---|---|---|
| Build | `cargo check --workspace --all-targets` | exit 0 |
| Tests | `cargo test --workspace` | pass |
| Lookbook | (use termrock's documented lookbook runner — check its README/COMPONENTS.md) | new story renders |
| Bench | `cargo bench` or the repo's perf-test convention — discover before writing | budget met |

| Purpose | Command (in tablerock) | Expected |
|---|---|---|
| Pin bump build | `cargo check --workspace --all-targets` | exit 0 |
| Full tests | `cargo test -p tablerock-tui -p tablerock-cli` | pass |

## Scope

**In scope**:
- TermRock repo: new `crates/termrock/src/widgets/virtual_grid.rs` (+ module
  registration, COMPONENTS.md entry, lookbook story, docs page if the repo
  keeps docs/content, tests, benchmark).
- TermRock repo: grid scrollbar state if absent (check `scroll` module first
  — reuse if it already models unknown totals).
- TableRock repo: `Cargo.toml` termrock rev bump + `Cargo.lock` — nothing
  else.

**Out of scope**:
- ANY database vocabulary in TermRock (values, pages, editability) — the
  grid renders caller-projected `&str`/styled cells only.
- TableRock's `DataGridModel` (plan 009).
- `TextArea`/`CompletionMenu` (plan 010).
- Modifying Jackin.

## Git workflow

Both repos are trunk-only, no branches/PRs, DCO (`git commit -s`), push
immediately (`AGENTS.md` applies to required TermRock changes too:
"These rules also apply to required TermRock changes"). TermRock commits
first; TableRock pin bump is a separate later commit referencing the exact
TermRock revision.

## Steps

### Step 1: Survey TermRock conventions

Read TermRock's `COMPONENTS.md`, an existing stateful widget pair
(recommend `tree.rs` — closest interaction model: stable IDs, disclosure,
keyboard/mouse) and its tests + lookbook story. Extract: state-struct
naming (`XState`), render signature, hit-region publication pattern,
event-handling conventions, test harness idioms. Write a short design note
(commit message or repo docs, per TermRock convention) mapping the required
contract to that idiom.

**Verify**: you can name, for each of the 9 gate requirements, where the
exemplar widget satisfies it.

### Step 2: Implement `VirtualGrid` + `VirtualGridState`

Contract (from the table above, expanded):
- Caller supplies per-frame: visible window request → callback/slice of
  borrowed cell projections (`&str` + style + optional glyph), row IDs
  (stable, u64/opaque), column specs (id, header, width, min-width).
- State owns: two-axis viewport (first row/col + offsets), selection
  (cursor cell + optional range anchor), column widths (caller-persisted —
  expose get/set), focus, scrollbar state with known/unknown totals.
- Behavior: keyboard (arrows/page/home/end both axes, selection extend),
  mouse (click select, drag range, wheel both axes, header hit regions,
  scrollbar drag), placeholders for cells outside the caller's resident
  window (caller returns `Pending` marker per cell — the grid renders it
  distinctly; it never fetches).
- No I/O, no async, no allocation of the full data set; render cost bounded
  by viewport size.
- Unicode display-width safe (reuse termrock text-width helpers).

**Verify**: `cargo test --workspace` in termrock — new behavior tests pass
(keyboard/mouse/focus/empty/clipping/min-rect/Unicode per the gate).

### Step 3: Lookbook + benchmark + docs

Deterministic lookbook story (large synthetic corpus, e.g. 1M virtual rows ×
40 cols with a windowed provider). Benchmark proving render of a
representative viewport (e.g. 60×200 cells) within the repo's stated hot-path
budget; record numbers in the commit/evidence. Caller-owned-policy docs
(fetching, editing, sort/filter all caller-side).

**Verify**: lookbook renders; bench numbers recorded.

### Step 4: Jackin compatibility + TermRock push

If any existing API changed (it should not — additive widget), run the
Jackin compatibility check per gate requirement 7 (Jackin pins an older
baseline; additive changes are compatible by construction — state this).
Commit(s) to TermRock `main`, DCO, push.

**Verify**: TermRock `main` CI/tests green at the new revision.

### Step 5: TableRock pin bump

Update `Cargo.toml` termrock `rev` to the new TermRock commit; refresh
version if TermRock bumped it; `cargo update -p termrock` to sync the lock.
Evidence doc in TableRock (TermRock T2 checkpoint complete, revision,
contract summary, bench numbers) + evidence-index line + termrock evidence
group. No TableRock code uses the grid yet (plan 009 does).

**Verify (tablerock)**: `cargo check --workspace --all-targets && cargo test -p tablerock-tui -p tablerock-cli` → pass.

## Test plan

TermRock-side behavior tests per the gate list; the million-row synthetic
viewport benchmark (ledger acceptance evidence); TableRock-side: existing
suites only (pin bump must be behavior-neutral).

## Done criteria

- [ ] `VirtualGrid` on TermRock `main` satisfying all 9 gate requirements (map each in the evidence doc)
- [ ] Bench: bounded viewport render cost recorded; million-row synthetic corpus story exists
- [ ] TableRock pinned to the exact new revision; workspace builds + tests green
- [ ] Evidence doc + index line in TableRock; `plans/README.md` updated

## STOP conditions

- TermRock `main` already contains a grid/virtual-grid widget — STOP,
  re-scope to adoption.
- The required contract conflicts with an existing TermRock convention
  (e.g. hit-region model can't express per-cell regions) — STOP; that is an
  upstream design decision.
- Jackin compatibility genuinely breaks — STOP (gate requirement).

## Maintenance notes

- Plan 009 composes `DataGridModel` over this; selection/columns API changes
  after 009 lands require Jackin-style compatibility discipline.
- Reviewer: zero product vocabulary in the TermRock API; render cost
  independent of total row count.
