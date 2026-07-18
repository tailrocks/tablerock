# Plan 011: SQL editor workbench — multiline editing, statement selection, completion, history, saved queries, files (Phase 5 core)

> **Executor instructions**: Work-package plan. Read
> `docs/product/sql-editor.md` (authority for every behavior) and
> `docs/architecture/fixed-decisions.md` "SQL/editor path" before starting.
> Follow the checkpoint order; each checkpoint is a trunk commit with
> evidence. STOP conditions binding. Update `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat d8b113b..HEAD -- crates/`
> plus confirm plans 009 and 010 are DONE in `plans/README.md`.

## Status

- **Status**: DONE (evidence 217–222; residuals: file path picker UI, external-change confirm dialog, find/replace/format/explain deferred by plan)
- **Priority**: P1
- **Effort**: L
- **Risk**: MED
- **Depends on**: plans/009, plans/010
- **Category**: direction (Phase 5 roadmap)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

Phase 5 turns the tracer into a workbench: real multiline editing, safe
statement boundaries, schema-aware completion, history, saved queries, and
query files. Everything here is shared by ClickHouse (plan 014) and mirrored
by Redis (plan 015).

## Fixed decisions that bind this plan

- `sqlparser` for PostgreSQL/ClickHouse tokens + last-known-valid AST;
  dialect-aware statement boundaries with token fallback for incomplete
  input; **never naive semicolon splitting** (fixed-decisions.md "SQL/editor
  path"). `sqlparser` is a NEW dependency: record version/features/license/
  MSRV/motivation per CONTRIBUTING.md in its adoption checkpoint, latest
  stable, exact pin.
- Completion revisioned against editor text revision + context revision +
  catalog generation; stale candidates never apply
  (`docs/product/sql-editor.md` "Autocomplete").
- History: bounded, searchable, configurable SQL-text retention + private
  mode (persistence: new tables — never persist result payloads;
  fixed-decisions.md "Persistence").
- Editor primitives are TermRock `TextArea`/`CompletionMenu` (plan 010);
  parser/candidates/diagnostics/execution live in TableRock
  (`QueryEditorModel`, termrock-integration.md local-model table).

## Current state (entry gate)

- Plan 009 shipped: grid, Execute path, single-line SQL tab, cancel, errors.
- Plan 010 shipped: contract-complete `TextArea` + `CompletionMenu`.
- Catalog snapshots with revisions exist per context (plans 003/007).
- Persistence actor + migration machinery exist (plan 004 pattern for new
  tables: `crates/tablerock-persistence/migrations/`, prefix-validated
  ledger `lib.rs:394-433`).

## Commands

Standard set: `cargo test -p tablerock-tui -p tablerock-cli -p tablerock-persistence`,
engine suites under Docker, `cargo clippy --workspace --all-targets`,
CI green after each push.

## Scope

**In scope** (checkpoints, each its own commit + evidence doc):

1. **Parser service** (new `crates/tablerock-engine/src/sql_analysis.rs` or a
   new `tablerock-sql` crate — prefer a module first; a crate needs a
   workspace decision): sqlparser adoption; statement-boundary API
   (`statements(text) -> Vec<StatementSpan>` with dialect + incomplete-input
   token fallback); syntax span classification for highlighting; tests over
   procedures/comments/strings/dollar-quoting/incomplete corpus
   (parity-ledger "Statement selection" acceptance).
2. **Multiline editor tab**: replace the 009 single-line input with
   `TextArea`-backed `QueryEditorModel`: spans computed outside render,
   current-statement marking, run-selection-else-current, per-tab
   text/cursor/results/errors (spec "Editing" + "Layout": editor above
   results, resizable remembered split via `SplitPane`).
3. **Completion**: candidate service combining catalog snapshot (revisioned)
   + keywords + functions + aliases parsed from text; `CompletionMenu`
   projection; commit replaces the correct token range; stale-revision
   rejection tests (edit-during-fetch, context-switch-during-fetch).
4. **History**: new migration (bounded history table: statement text
   optional by retention policy, engine, context facts, timestamps, outcome
   class); actor API append/search/list; TUI history panel; retention +
   private mode settings; redaction rule — history is SQL text, so the
   store must honor "configurable SQL retention/private mode"
   (delivery-plan Phase 5) with tests.
5. **Saved queries + files**: named saved queries (persistence) + `.sql`
   file open/save with atomic write (temp+rename), external-change
   detection (mtime check on focus/interval effect), unsaved-change policy
   through the single dialog authority (plan 007's).
6. **Session restoration (intent-only)**: persist open tabs/context/editor
   text per profile (never results/pending writes — fixed decision);
   restore on connect.

**Out of scope**: find/replace, formatting, explain, parameters (all
Phase 5 ledger rows deliverable AFTER this plan as small follow-ups —
record as visible gaps), Vim mode (Phase 10), Redis command editor (plan
015), multi-statement result sections beyond what plan 009's grid already
shows sequentially (full multi-result tabs land in plan 016).

## Steps / verification

Each checkpoint: implement → tests → evidence doc + index → ledger row
update → commit/push → CI green. Named verification highlights:

- Parser: corpus test with `$$` bodies, nested comments, `E''` strings,
  emoji identifiers; incomplete input never panics
  (`cargo test -p tablerock-engine --lib sql_analysis`).
- Editor: reducer tests for run-selection vs current-statement; render
  tests with syntax spans; IME-like paste corpus (delivery-plan Phase 5
  exit: "Unicode/IME-like paste cases").
- Completion: race tests (three: text-stale, context-stale,
  catalog-stale); injection test — completing after `where name = '` never
  executes anything.
- History: retention modes (full/off/private) with DB-inspection
  assertions; bounded size enforcement.
- Files: atomic-save crash test (kill between temp write and rename →
  original intact); external-modification prompt test.
- Restoration: relaunch harness proves intent-only (no result rows in DB —
  schema has nowhere to put them; assert tables).

## Done criteria

- [x] Statement selection never uses naive splitting (`sql_analysis` / `QueryEditorModel.run_text`)
- [x] Completion stale-rejection proven for all three revision axes (evidence 219)
- [x] History retention/private modes proven by DB inspection (evidence 220)
- [x] Atomic save + external-change detection proven (evidence 222)
- [x] Restoration restores tabs/text/context; never results (evidence 222)
- [x] `sqlparser` adoption checkpoint records version/license/MSRV/motivation (217)
- [x] Evidence + plans/README updated; ROADMAP Phase 5 partial

## STOP conditions

- sqlparser's ClickHouse dialect cannot produce usable boundaries for a
  required statement class — STOP; record the gap (the fixed decision names
  sqlparser; changing it is an architecture revision).
- History schema wants result payloads for "outcome" display — STOP; only
  outcome classes/counts are permitted.
- A new crate split is needed — STOP; workspace layout is an architecture
  decision.

## Maintenance notes

- Plan 014 reuses parser+completion with ClickHouse dialect; plan 015
  mirrors the editor shape with Redis command metadata; plan 016 builds
  multi-statement result tabs on the boundary API.
- Deferred explicitly: find/replace, formatting, explain, bound parameters —
  visible parity-ledger gaps after this plan.
