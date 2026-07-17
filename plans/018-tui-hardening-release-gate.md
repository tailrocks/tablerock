# Plan 018: TUI hardening and the parity release gate (Phase 11)

> **Executor instructions**: Work-package plan. Authority:
> `docs/architecture/quality-and-verification.md` (READ IT FULLY — it is the
> gate definition, 408 lines), delivery-plan.md "Phase 11", parity ledger
> "Closure rule". STOP conditions binding. Update `plans/README.md` when
> done.
>
> **Drift check (run first)**: plan 017 DONE; ledger has no silently-open
> Core rows.

## Status

- **Priority**: P2
- **Effort**: L
- **Risk**: MED
- **Depends on**: plans/017
- **Category**: direction (Phase 11 roadmap — TUI release gate)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

Phase 11 is the gate between "features exist" and "terminal product
release". Its exit rule: every parity-ledger row is implemented, explicitly
excluded, or visibly blocks the parity claim — only the first two states
permit the claim.

## Deliverables (from delivery-plan Phase 11, expanded to work items)

1. **Suite completion audit**: map every suite class —
   unit/model/adapter/integration/render/PTY/real-server — against actual
   coverage; fill gaps. The PTY harness exists
   (`crates/tablerock-cli/tests/pty_lifecycle.rs`); extend it to drive real
   product flows (connect→browse→query→quit) against containerized servers.
2. **Failure injection**: systematic matrix — disconnect mid-stream,
   timeouts, cancellation races, disk full (persistence + export), Turso
   migration failure, terminal failure (SIGWINCH storms, PTY close),
   partial database outcomes. Persistence fault gaps recorded in evidence
   58/62/135 (disk-full, backup-publication faults, operator replacement
   UX) MUST close here or block the claim.
3. **Performance budgets on the support matrix**: measured startup,
   first-row, resident-scroll, completion latency, memory, shutdown —
   release-profile artifacts (evidence 133 lists release-profile + cold/warm
   startup + TUI scrolling as open). Promote `performance_real` into a
   scheduled CI job on fixed-spec runners or a documented local rig; record
   numbers as budgets.
4. **Accessibility/i18n audits**: terminal accessibility (screen-reader
   conventions where applicable), keyboard-complete + mouse parity,
   non-color state cues (audit every state render), Unicode width/combining
   corpus across grid/editor/tree, narrow-layout completeness, restoration
   audit.
5. **Telemetry**: local `tracing` with safe schemas; optional OTLP export
   disabled by default with the fixed safe schema (IDs, engine, safe codes,
   durations, counts, transitions — fixed-decisions.md "Telemetry"); an off
   path with zero background sockets (test).
6. **Provenance/license/secret audit**: clean-room provenance records
   complete per influenced commit; `cargo deny` clean; secret/log audit —
   grep-based + review: no SQL/args/values/credentials in any log, Debug,
   telemetry, or crash path (extend the existing redaction tests into a
   workspace-wide audit test).
7. **Ledger closure pass**: row-by-row audit producing the three-state
   classification; publish the support matrix (exact server versions,
   terminals, platforms).

## Commands

Everything from plans 001–017 plus: release-profile builds
(`cargo build --release`), the PTY product-flow suite, scheduled perf runs.
CI: add release-build job + scheduled perf workflow (SHA-pinned actions,
freshness-checked per repo convention).

## Done criteria

- [ ] quality-and-verification.md satisfied item-by-item (evidence doc maps each requirement → proof)
- [ ] Failure-injection matrix green incl. the persistence fault gaps (58/62/135)
- [ ] Release-profile budget numbers recorded on the published support matrix
- [ ] Non-color cue audit: zero states conveyed by color alone (audit doc)
- [ ] OTLP off-by-default proven (no sockets test); safe schema enforced by type
- [ ] Ledger: three-state closure complete; parity claim status explicit
- [ ] ROADMAP Phase 11 complete; `plans/README.md` updated

## STOP conditions

- Any ledger row is tempting to mark "implemented" without its acceptance
  evidence — STOP; the closure rule forbids it.
- Perf budgets fail on release artifacts — STOP; optimization is its own
  planned work, not a gate fudge.
- A failure-injection case reveals ambiguous-write replay anywhere — STOP;
  that is a P1 defect above this plan.

## Maintenance notes

- This plan's audit artifacts (matrix, budgets, audit docs) become the
  regression baseline for plans 019–021 (native work must not regress
  them).
