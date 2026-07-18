# Plan 010: TermRock T3 — audit `TextArea` against the required contract and contribute `CompletionMenu`

> **Executor instructions**: Two repositories again (see plan 008 preamble —
> same workflow, same contribution gate). STOP conditions binding. Update
> `plans/README.md` when done.
>
> **Drift check (run first)**: confirm TableRock's termrock pin (Cargo.toml
> `termrock` rev — after plan 008 it moved past `b7f34da`); survey
> `crates/termrock/src/widgets/` in that revision for `completion`-like
> widgets before writing code.

## Status

- **Status**: DONE (TermRock `dd8bed1` CompletionMenu; TextArea audit residual gaps recorded in evidence 216)
- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: plans/008 (workflow precedent; independent otherwise — can run in parallel with 009)
- **Category**: direction (TermRock checkpoint T3)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

Phase 5 (SQL/Redis editor, plan 011) requires two neutral primitives. At rev
`b7f34da` TermRock ships `text_area.rs` (+ `edit_core.rs`) but NO completion
widget. The delivery sequence defines T3: "`TextArea` and `CompletionMenu`
with Unicode/editor corpus — unblocks SQL/Redis workbench". `TextArea`
existing does not mean it satisfies the contract — the roadmap (Phase 5)
still says "Add TermRock `TextArea` and `CompletionMenu`", so audit first,
extend where short, contribute the menu.

## Current state

- Required `TextArea` contract (delivery-plan.md "TermRock T3" +
  termrock-integration.md primitives table): grapheme-safe editing,
  cursor/selection, undo/redo, line numbers, search, vertical+horizontal
  scroll, paste, external spans/diagnostics (caller-computed syntax
  highlighting overlay), geometry clamping, stable candidate IDs
  (CompletionMenu), lookbook/Buffer tests, Jackin compatibility. "Parser,
  ranking, database policy, and execution remain in TableRock."
- Required `CompletionMenu` contract (termrock-integration.md table):
  "Stable candidates; selected ID; clamp/flip geometry; scroll;
  keyboard/mouse; caller ranking and commit". Product behavior it must
  support (`docs/product/sql-editor.md` "Autocomplete"): popup anchored at
  cursor, never covers the cursor, flips/clamps inside the editor,
  Enter/Tab commits, Escape dismisses, keyboard+mouse navigation.
- TermRock files at `b7f34da`: `widgets/text_area.rs`, `widgets/edit_core.rs`
  (shared editing core), `widgets/picker.rs`, `widgets/selection.rs`
  (nearest existing primitives to a completion menu).

## Commands you will need

Same two-repo command tables as plan 008 (check/test/lookbook/bench in
termrock; pin-bump check/test in tablerock).

## Scope

**In scope**:
- TermRock: gap audit of `TextArea`/`edit_core` vs the contract; additive
  extensions for any missing item (likely candidates: external
  span/diagnostic overlay API, search, undo/redo depth policy, horizontal
  scroll — verify each against actual source before assuming);
  new `widgets/completion_menu.rs` per the contract; lookbook stories,
  behavior tests, docs, COMPONENTS.md rows.
- TableRock: pin bump + evidence doc only.

**Out of scope**:
- SQL parsing, candidate ranking, dialect logic (TableRock, plan 011).
- Vim mode (Phase 10; the neutral editor state machine must merely not
  preclude it).
- Rewriting `edit_core` architecture.

## Git workflow

Identical to plan 008: TermRock trunk-only DCO commits pushed to `main`,
then a TableRock pin-bump commit + evidence.

## Steps

### Step 1: `TextArea` contract audit

Read `text_area.rs` + `edit_core.rs` + their tests at the current pin. For
each contract item produce a PASS/GAP verdict with file:line evidence. Write
the audit into the TableRock evidence doc (it is TableRock's adoption
evidence). Items commonly under-specified — check explicitly: grapheme
clusters vs chars (test with ZWJ emoji + combining marks), display-width
(CJK), undo/redo grouping, bracketed-paste multi-line insert, external span
overlay (caller supplies byte-range → style without the widget re-parsing),
search API, viewport clamping at minimum rectangles.

**Verify**: audit table complete; every GAP has a file:line justification.

### Step 2: Close `TextArea` gaps (additive)

Implement each GAP in TermRock with behavior tests + lookbook updates.
API changes must stay compatible (gate requirement 7 — Jackin check).

**Verify**: termrock `cargo test --workspace` green.

### Step 3: `CompletionMenu`

New widget: candidate list with stable IDs (caller-owned ordering), selected
ID, anchor-point geometry with flip/clamp inside a caller-supplied bounding
rect, never covering the anchor cell, scroll, keyboard (up/down/page,
commit/dismiss are CALLER-mapped — widget exposes semantic outcomes),
mouse (hover select, click commit), disabled/empty states, Unicode-width
labels + optional right-aligned kind annotation. Model after `picker.rs`
idioms. Tests per the gate (incl. clamped geometry at screen edges — the
product rule "popup never covers the cursor and flips/clamps" must be
provable here).

**Verify**: termrock tests + lookbook story green.

### Step 4: Pin bump + evidence (TableRock)

Bump rev, lock, evidence doc (audit table + menu contract + Jackin note),
index line.

**Verify (tablerock)**: `cargo check --workspace --all-targets && cargo test -p tablerock-tui -p tablerock-cli` → pass.

## Test plan

TermRock behavior tests (keyboard/mouse/focus/empty/clip/min-rect/Unicode)
for every extension + the new menu; deterministic lookbook stories; TableRock
suites unchanged-green on pin bump.

## Done criteria

- [x] Audit table in evidence 216 (PASS items + residual GAP rows for selection/undo/line numbers/search/spans)
- [x] `CompletionMenu` on TermRock `main` (`dd8bed1`) meeting gate requirements
- [x] Geometry tests prove flip/clamp/never-cover-anchor
- [x] TableRock pinned to new revision, suites green
- [x] Evidence + index updated; `plans/README.md` updated

## STOP conditions

- `TextArea`'s architecture cannot support external span overlays without
  breaking existing consumers — STOP (upstream design decision).
- A completion-like widget already exists upstream — STOP, re-scope to
  audit/extend.

## Maintenance notes

- Plan 011 builds `QueryEditorModel` + candidate services on these; commit
  semantics (replace-token range) live TableRock-side — the menu only
  reports "candidate X committed".
- Reviewer: no parser, no ranking, no database words in TermRock APIs.
