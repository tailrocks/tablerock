# Plan 021: Native workflow parity, release evidence, and parity closure (Phases 14–15)

> **Executor instructions**: Work-package plan covering the final two roadmap
> phases. Authority: delivery-plan.md "Phase 14"/"Phase 15", ROADMAP 14–15,
> parity-ledger "Native macOS parity" + "Closure rule",
> `docs/product/native-macos.md`. STOP conditions binding. Update
> `plans/README.md` when done.
>
> **Drift check (run first)**: plan 020 DONE.

## Status

- **IN PROGRESS (2026-07-19): native connection workflows (evidence 512–518), history/retention (519–520), saved queries (521), SQL files (522), typed intent bridge (523), query tabs (524), read-only preview/pinned object tabs (525), multi-window restoration (526), environment/safety projection (527), typed value inspection (528), shared Rust/native multi-format copy (529), native atomic loaded-result export (530), shared bounded CSV import foundation (531), native reviewed CSV import (532), PostgreSQL/ClickHouse structure (533–534), bounded Redis key catalog projection (535), and native Redis key object views (536) landed; broader import types/engines, full streaming export, advanced object state, and remaining screens continue**
- **Priority**: P3
- **Effort**: L
- **Risk**: MED
- **Depends on**: plans/020
- **Category**: direction (Phases 14–15 roadmap)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Phase 14 — native workflow parity (checkpoint groups)

1. **Screen completion**: every product-spec screen's native projection —
   connection organization (groups/tags/search), workbench tabs
   (`NSWindowTabbing` semantics with the preview/durable rules unchanged),
   editors, grids, inspectors, history/saved/files, edit review, data
   movement (`NSSavePanel`/`NSOpenPanel` + security-scoped access), Redis
   and ClickHouse engine-specific screens. Each screen's "Both clients"
   table row is its acceptance spec.
2. **Platform behavior**: multi-window ownership/restoration over shared
   Rust sessions, menus/commands completeness, drag/drop + pasteboard
   (multiple representations from the plan-012 formatters), settings,
   native appearance (light/dark/accent), IME/marked text.
3. **Accessibility matrix**: VoiceOver, keyboard, focus, selection, marked
   text/IME, reduced motion, contrast, large content — tested per
   delivery-plan Phase 14.
4. **Release evidence**: signing, hardened runtime, notarization/stapling
   on the full app, credentials (Keychain + 1Password CLI when staged),
   update/migration (persisted-store schema migration on app upgrade),
   crash recovery, uninstall residue audit.
5. **Semantic equivalence**: native and TUI produce semantically equivalent
   Rust outcomes for every shared workflow — extend the plan-019
   conformance suite with workflow-level scripts (same commands → same
   events/pages/outcomes through both adapters).

## Phase 15 — closure and maintenance (checkpoint groups)

6. **Final ledger audit**: every row → implemented (tests + user docs
   linked) / excluded (decision linked) / visible gap (blocks the claim).
   Release claims list exact engines, server versions, platforms,
   cancellation limitations, distribution shape, exclusions.
7. **User documentation**: per-capability docs matching actual behavior
   (the delivery rule "documentation and support claims match the result").
8. **Support matrix + diagnostics**: tested server/terminal/macOS/
   architecture/migration matrix in CI; support diagnostics bundle with
   redacted failure collection (safe-schema only).
9. **Provenance/license/reproducibility/release audit**: clean-room
   provenance completeness, license inventory, reproducible builds where
   claimed, artifact checksums.
10. **Compatibility monitoring**: recurring CI verification for TermRock,
    Ratatui, database clients, Rust, Swift, macOS, servers, 1Password CLI,
    packaging tools (extends the existing freshness workflow); forward-only
    small-commit maintenance policy documented.

## Commands

Everything from plans 018–020 (both toolchains), plus the workflow
conformance scripts and the documentation build if one exists. Evidence per
checkpoint, ledger update per exit.

## Done criteria

- [ ] Every product screen exists natively per its "Both clients" row
- [ ] Workflow-equivalence suite green (same Rust outcomes both clients)
- [ ] Full accessibility matrix recorded
- [ ] Clean-machine Release artifact passes install/update/uninstall/crash-recovery audits
- [ ] Ledger closure: no silently-open row; release claims exact
- [ ] Compatibility monitoring running on schedule
- [ ] ROADMAP Phases 14–15 complete; `plans/README.md` updated

## STOP conditions

- Any workflow's native outcome diverges semantically from the TUI — STOP;
  that is a Rust-contract bug, not a Swift patch site.
- A ledger row pressures "implemented" without user docs + tests — STOP
  (closure rule).
- Platform-only behavior starts accumulating domain logic — STOP.

## Maintenance notes

- This plan ends the program; what follows is the documented forward-only
  compatibility maintenance loop (group 10).
