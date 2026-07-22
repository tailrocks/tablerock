# Plan 016: Daily workflows and data movement — result tabs, import/export, preferences, resilience (Phase 9)

> **Executor instructions**: Work-package plan. Authority docs:
> `docs/product/copy-export.md`, delivery-plan.md "Phase 9",
> ROADMAP Phase 9. Trunk checkpoints with evidence. STOP conditions binding.
> Update `plans/README.md` when done.
>
> **Drift check (run first)**: plans 013, 014, 015 DONE.

## Status

- **Priority**: P2
- **Effort**: L
- **Risk**: MED (file effects are a new surface)
- **Depends on**: plans/013, plans/014, plans/015
- **Category**: direction (Phase 9 roadmap)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

Phase 9 makes the workbench daily-drivable: complete multi-statement result
handling, saved filters/preferences, streaming file import/export, and the
resilience rules (cancellation cleanup, one-failed-tab isolation, no
reconnect storms). It also introduces the product's FIRST file-writing
effects — the evidence ledger repeatedly lists "product file effects &
atomic destinations" as open (e.g. COPY doc 162).

## Spec anchors (inline)

- Result tabs: one result section per statement in order, own summary
  (rows/timing/command tag), failure doesn't hide earlier results
  (`sql-editor.md` "Execution and results"); pinning + completion summaries
  (delivery-plan Phase 9).
- Export: loaded result OR streaming full re-query → CSV/JSON with progress
  + cancel; atomic destinations — partial files from cancelled/failed
  exports are REMOVED; SQL INSERT dumps with bounded streaming
  (`copy-export.md` "Export").
- Import: CSV/JSON into a chosen table; column mapping, encoding handling,
  progress, cancel, explicit partial-import outcomes; formula-like content
  is data, never evaluated (`copy-export.md` "Import").
- Phase 9 exit: import/export bounded, malformed data/encoding/formula-safe;
  incomplete export files removed; permissions/unsupported visible; relaunch
  cannot cause reconnect storm or resurrect results/edits.

## Current state (entry gate)

- Multi-statement outcomes proven engine-side for PG (evidence 161:
  ordered outcomes, stable ordinals, honest kinds/counts); editor executes
  statements (011); grid + copy formatters (009/012); engine writes
  (013–015); PG COPY primitives proven driver-side (doc 162) but unwired.
- NO file-effect infrastructure: no path-picker UI, no atomic-write helper
  beyond the query-file save (011), no streaming file writer.
- Preferences: `ProfilePreferences` exists (core); no saved filters, no
  column/object preference store beyond plan 012's column layouts.

## Scope (checkpoints)

1. **Multi-statement result tabs**: per-statement result sections with
   ordered summaries + failure isolation; result-tab pinning; multi-
   operation completion summaries.
2. **File-effect foundation**: CLI-side `FileEffects` adapter — path
   validation, create-exclusive, temp+rename atomic policy, cleanup-on-
   cancel/failure registry, progress reporting into ingress; path-picker
   UI (TermRock `TextInput` + validation; native panels come with the
   macOS client).
3. **Export**: loaded-result export (reuse plan 012 formatters, streamed to
   file) + full re-query streaming export (bounded pages → encoder → file)
   for CSV/JSON, then SQL INSERT dumps; progress + cancel + partial-file
   removal proofs. PG COPY-based fast path may be used where semantics
   match (driver primitive exists) — else plain SELECT streaming; record
   choice.
4. **Import**: CSV/JSON reader (bounded, encoding-explicit, RFC-4180
   tolerant reader with error positions), column-mapping UI, type
   conversion via existing typed editors' parse rules, batched apply
   through the engine write seams (PG transactional batches, CH progressive
   insert, Redis n/a — unsupported state), explicit partial outcomes;
   formula-content neutrality test (`=SUM(...)` imports as text).
5. **Saved filters + preferences**: persist named filter presets per table
   (plan 012 builder plans), object/profile organization preferences,
   result-tab preferences; migration + actor API per plan-004 pattern.
6. **Resilience**: cancellation cleanup audit across all long operations;
   one-failed-tab isolation test matrix; relaunch behavior — restoration
   intent-only (011) + no automatic reconnect of previously-live sessions
   without explicit preference (`ReconnectPreference` exists in core);
   cache/eviction pressure test (ResultStore limits under many tabs).

**Out of scope**: cross-engine copy/movement UI (post-parity; support-matrix
documentation only per ROADMAP wording "cross-engine support documentation"),
pg_dump/restore (Phase 10), scheduled jobs (excluded).

## Commands

Standard suites + Docker engine suites; new import/export tests live in
`tablerock-cli` (file effects) + engine crates (streaming); CI updated.

## Done criteria

- [x] Multi-statement sections model: middle failure keeps 1st + 3rd explicit (unit test) — evidence 245
- [x] Export abort/drop removes partial temp (unit test); ExportResult effect wired — evidence 245
- [x] Import CSV: formula content as data, oversized/malformed errors with positions — evidence 245
- [x] Partial import apply: CSV→InsertRow + review/authorize/apply on PG
      (`apply_csv_inserts`, real test import_apply_real)
- [x] Streaming full re-query export + cancel cleanup (stream_export + effect)
- [x] Relaunch: Manual reconnect never auto (should_auto_reconnect test)
- [x] Saved filters JSON round-trip (in-memory library)
- [x] Suites green for landed checkpoints; plan index DONE

## Progress notes

- 245 file foundation + loaded export + CSV parse + result sections
- 246 saved filters + reconnect policy
- 644 constant-memory CSV batch scanner: 64 KiB read buffer, bounded cells and
  row batches, exact UTF-8/CSV positions, formula neutrality, and cancellation
  between batches. Native async apply/progress ownership remains the next
  checkpoint.
- 645 progress-aware native apply: CSV review tokens can only enter a Rust
  asynchronous operation; PostgreSQL/ClickHouse report row-boundary progress,
  cancellation truth, bounded safe errors, and terminal/partial summaries.
  Connecting scanner batches to frozen-file review remains open.
- 646 closes that native residual: SHA-256 preview binding, private frozen
  spool, constant-memory full typed validation, 500-row/8 MiB apply batches,
  PostgreSQL transactional and ClickHouse progressive live proof, progress,
  cancellation, bounded error copy, expiry/discard/terminal cleanup.

## Residual (non-blocking)

- ~~ClickHouse import apply batch~~ (closed: import_apply_real CH progressive)
- ~~Streaming re-query export with cancel mid-stream~~ (closed: stream_export)
- ~~Persistence actor API for filter presets~~ (closed: evidence 306–307, 310)
- ~~Multi-statement UI wiring into QueryEditorModel run path~~
  (closed: evidence 292 selection path + 318 explicit RunScript full buffer)
- ~~Fuzzy multi-preset filter picker~~ (closed: evidence 319 rank + unique resolve)
- ~~TUI Effect wiring for import apply / stream export actions~~
  (ImportCsv + ExportStreamCsv/Json/Tsv → effects; unit-tested)

## STOP conditions

- Atomic temp+rename can't hold on a target filesystem semantics question
  (cross-device rename) — implement copy+fsync+rename-in-dir instead;
  if neither provable, STOP.
- Import type conversion requires per-engine semantics the typed editors
  don't define — STOP; record the gap per type.
- Any import path builds SQL by string concatenation — STOP.

## Maintenance notes

- Phase 10 adds pg_dump/pg_restore over the same file-effect foundation —
  keep `FileEffects` engine-agnostic.
- Reviewer: fault-injection coverage (kill/cancel at every stage), bounded
  memory during streaming (no full-file buffering).
