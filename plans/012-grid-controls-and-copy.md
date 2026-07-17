# Plan 012: Grid controls — server sorting, filter bar, column management, copy formats (Phase 5 grid half)

> **Executor instructions**: Work-package plan. Read
> `docs/product/data-grid.md` and `docs/product/copy-export.md` first — they
> are the behavioral authority. Checkpoint order below; trunk commits with
> evidence. STOP conditions binding. Update `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat d8b113b..HEAD -- crates/`
> plus plans 009 (grid) DONE; 011 helpful but not required (parallel OK).

## Status

- **Priority**: P1
- **Effort**: L
- **Risk**: MED (SQL construction safety)
- **Depends on**: plans/009
- **Category**: direction (Phase 5 roadmap)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

Browsing without sort/filter/column control is a demo, not a workbench.
This plan also establishes the **typed query-plan builder** — the only place
in the product that composes SQL from UI state — whose injection safety every
later phase reuses (editing WHERE clauses, FK navigation, import/export).

## Spec anchors (inline; executor must still read the docs)

- Sorting (`data-grid.md`): header cycles asc/desc/none; second column =
  tie-breaker with visible order index; server-side with parameterized
  identifiers — never string concatenation; provenance in status bar;
  cleared on reset.
- Filtering: filter bar with two modes — typed column conditions
  (operator list typed to column type, AND-combined, re-runs query) and raw
  WHERE fragment (appended, parameterized where it references values,
  hostile fragments fail closed); removable chips + clear-all; separate
  quick filter over resident rows only, visibly labeled page-local.
- Columns: show/hide/reorder/resize; one-action reset; layout persists per
  table across sessions; hiding never changes query identity/editability.
- Copy (`copy-export.md`): scopes cell/cells/rows/whole loaded result;
  formats CSV (RFC-4180), TSV (no quoting), JSON (typed where
  representable), SQL INSERT/UPDATE (require base-table identity facts —
  absent otherwise, never degraded), Markdown pipe table; binary/NULL/
  truncated/unknown copy with explicit representations, truncated marked.
- Ledger acceptance: hostile identifier/type fixtures (Sorting);
  parameterization, operator-per-type, NULL semantics (Filtering);
  clipboard-neutral formatter tests incl. identity-gated SQL (Selection/copy);
  narrow/wide/Unicode geometry + persistence (Column controls).

## Current state (entry gate)

- Plan 009's `DataGridModel` + browse statement path + `Execute` intent.
- Persistence migration machinery for per-table column-layout storage
  (pattern: plan 004's migration 0007).
- `quote_ident` helper exists engine-side (plan 009 Step "Table browsing").
- Clipboard: NO adapter exists yet — `Effect` vocabulary gains a
  `CopyToClipboard` effect; CLI implements an OSC 52 writer through the
  TermRock session's typed OSC request surface (termrock-integration.md
  lists "typed OSC requests" under the session adapter; verify the API at
  the current pin — if absent, TermRock-first contribution, STOP condition).

## Scope (checkpoints)

1. **Typed browse-plan builder** (engine side, new module
   `crates/tablerock-engine/src/browse_plan.rs`): owned
   `BrowsePlan { table, sort: Vec<SortKey>, filters: Vec<TypedCondition>, raw_where: Option<BoundedText>, page }`
   → parameterized SQL + typed parameter values. Identifiers via
   `quote_ident`; values ALWAYS `$n` parameters; raw WHERE wrapped as
   `(...)` AND-composed; plan Debug redacts values. Adversarial tests:
   identifier `"; DROP TABLE x; --`, operator/type mismatches rejected
   pre-SQL, raw fragment with `$1` collisions renumbered or rejected
   (choose + document), NULL semantics (`IS NULL` operators distinct from
   `= NULL`).
2. **Sorting UI**: header interaction on `VirtualGrid` (hit regions from
   plan 008), sort state in `DataGridModel`, re-run via plan builder,
   provenance in status bar.
3. **Filter bar**: condition rows (column picker/operator/value input typed
   per `ValueKind`/engine type), chips, clear-all, raw-WHERE mode with
   fail-closed error surface, page-local quick filter labeled distinctly.
4. **Column management**: show/hide/reorder/resize/reset in
   `DataGridModel` + `VirtualGrid` widths; persistence: new migration for
   per-table layout keyed by (profile, database, schema, table), actor
   get/set API; restore on open.
5. **Copy formats**: Rust-owned formatter module (engine or core-adjacent;
   pure functions over `ResultPage` + selection): all six formats,
   identity-gating for INSERT/UPDATE (base-table identity facts arrive from
   the browse plan — a plain SELECT result has none), explicit
   binary/NULL/truncated representations; `CopyToClipboard` effect + OSC 52
   adapter + format picker UI.

**Out of scope**: saved filter presets (Phase 9), file export (plan 016),
editing (plan 013), FK navigation (plan 013).

## Commands

Standard: TUI/CLI/engine test suites, Docker PG suite for end-to-end
sort/filter tests, clippy, CI.

## Verification highlights

- Plan-builder fuzz/adversarial suite is the heart: hostile identifiers,
  hostile raw fragments, every operator×type cell (table-driven).
- Real-server: sort+filter round-trip on fixture table incl. Unicode column
  names and case-sensitive identifiers; NULL filter semantics.
- Copy: golden-file tests per format; INSERT/UPDATE absent without identity
  (test asserts absence, not error); clipboard effect emits OSC 52 payload
  (assert via session test writer).
- Column persistence: relaunch harness restores layout; reset restores
  defaults.

## Done criteria

- [ ] No SQL string concatenation of user values anywhere (`grep -rn "format!(" crates/tablerock-engine/src/browse_plan.rs` shows identifiers-only usage; adversarial suite green)
- [ ] Sort/filter/columns behave per spec at all layouts (render + reducer tests)
- [ ] Quick filter provably page-local (no effect emitted; label rendered)
- [ ] Six copy formats golden-tested; SQL formats identity-gated
- [ ] Column layout persists per table across relaunch (test)
- [ ] Suites + CI green; ledger rows (Sorting, Filtering, Column controls, Selection/copy) + evidence updated; `plans/README.md` updated

## STOP conditions

- TermRock session lacks a typed OSC 52/clipboard request — STOP; upstream
  contribution first (contribution gate applies).
- Raw-WHERE parameter renumbering proves ambiguous with the chosen client
  API — STOP and report options (reject vs rewrite).
- Base-table identity facts require catalog data plan 003 doesn't provide
  (pk/unique columns) — extend plan 003's relation listing with key facts in
  a small preliminary checkpoint; if that snowballs, STOP.

## Maintenance notes

- Plan 013 (editing) consumes identity facts + plan builder; plan 014/015
  reuse the formatter; plan 016 reuses formats for file export.
- Reviewer: the plan builder is the security boundary — demand the
  adversarial suite before UI review.
