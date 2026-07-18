# Plan 020: Native macOS vertical slice — SwiftUI/AppKit Liquid Glass over the bridge (Phase 13)

> **Executor instructions**: Work-package plan. Authority:
> `docs/product/native-macos.md` (design language — READ FULLY),
> `docs/architecture/native-macos-path.md`, delivery-plan.md "Phase 13",
> every product screen's "Both clients" table. STOP conditions binding.
> Update `plans/README.md` when done.
>
> **Drift check (run first)**: plan 019 DONE with all gates green.

## Status

- **Checkpoints 1–7 prototype complete (2026-07-18) + behaviorally verified
  (2026-07-19); plan remains IN PROGRESS**
  — behavioral prototype originally delivered via Command Line Tools only.
  Full Xcode 26.6 is now installed; Developer ID remains absent. The local build uses direct `swiftc`
  (`scripts/build-native-app.sh`), not SwiftPM: swiftc links the cargo release
  dylib + SwiftUI/AppKit from the macOS 26.5 SDK, producing a runnable
  `TableRock.app`. Checkpoints: cp1 shell+bridge · cp2 connection list
  (`list_profiles`) · cp3 connect (`open_profile`) · cp4 catalog browse
  (operation/event/page flow) · cp5 grid (full page-body decode: columns +
  text/NULL/signed/unsigned cells) · cp6 SQL query editor (execute) · cp7
  edit/review (`stage_probe_review` → authorize → apply). Behavioral
  verification (`scripts/verify-native-behavior.sh`) round-trips a real query
  through the bridge + page decode against all three engines (PostgreSQL,
  ClickHouse, Redis — live Docker); it caught + fixed 3 real decoder bugs
  (missing `pump`, warnings u32→u16, integers LE→BE). **Gate resolution:**
  the plan-019 distribution gate gates the *notarized XCFramework release*
  (Developer ID — operator); a *workable local app* does not
  require it. The prototype does not yet satisfy all of the plan's
  accessibility, appearance, lazy-catalog, or Instruments done criteria.
  Evidence 407 removes `ObservableObject` and adds the required
  Settings scene. Evidence 408 adds actor-owned bridge I/O, off-main pump/page
  decode, operation-ID cancellation UI, and a strict Swift 6 build gate;
  evidence 409 replaces the SwiftUI result renderer with the required reusable
  AppKit `NSTableView`; evidence 410 replaces `TextEditor` with the required
  IME-safe AppKit `NSTextView`/TextKit editor. Evidence 411 adds the AppKit
  `NSOutlineView` catalog and removes engine-specific catalog SQL from Swift
  behind a Rust-owned typed intent. Evidence 412 adds focused per-window Query
  commands, customizable toolbar actions, and corrects every native build
  surface to the fixed macOS 26 deployment target. Evidence 413 fixes bridge
  mutex starvation and proves live slow-query cancellation through the strict
  Swift path. Evidence 414 adds a reproducible eight-variant appearance fixture,
  captured artifacts, and a structural custom-control accessibility gate.
  Remaining system-setting/VoiceOver criteria stay open.
- **Priority**: P2
- **Effort**: L
- **Risk**: MED
- **Depends on**: plans/019
- **Category**: direction (Phase 13 roadmap)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

First native client: one window per connection session, connect → browse →
query → page → cancel → one reviewed safe operation, on every applicable
engine — with Swift containing ZERO database/safety behavior. Phase 13 exit
(delivery-plan): "Swift contains no driver, parser safety, edit-plan,
redaction, or result-authority duplication; there is no per-cell bridge
call."

## Design-language rules (inline from docs/product/native-macos.md)

- Target macOS 26 Tahoe+; Liquid Glass native from the start (macOS 27
  removes the compatibility opt-out; no legacy-material mode).
- Glass: window toolbar (context bar), sidebar, sheets/popovers/completion/
  review dialogs. NEVER glass: data grid, SQL editor, text content — opaque,
  scrolling UNDER the glass layer with hard scroll-edge effect.
- One glass cluster per region (`ToolbarSpacer` groups; one
  `GlassEffectContainer` per custom cluster); never glass-on-glass; tint
  only primary actions (Run, Apply) with `.regular.tint`.
- No custom blurs, no `NSVisualEffectView` sidebar material, no custom
  toolbar backgrounds.
- Production environment marking = label + accent on the toolbar badge,
  never translucency/color alone.
- Reduce Transparency / Increase Contrast / Reduce Motion degrade
  gracefully — behavior and legibility never depend on the material.
- Structure: SwiftUI `App`/`WindowGroup` (one window per session,
  multi-window), `Commands` via `@FocusedValue`, `Settings` scene;
  `NavigationSplitView`; AppKit via `NSViewRepresentable` for
  `NSOutlineView` (catalog), `NSTableView` (grid), `NSTextView`/TextKit
  (editor); `.toolbar(id:)` user-customizable; `controlSize(.small)`
  density in inspectors; native `NSWindowTabbing` for tabs; SF Symbols with
  restrained effects.
- Swift ownership: `@Observable` `@MainActor` classes holding immutable
  Rust snapshots (no `ObservableObject`); actor-owned bridge client polls
  off-main and publishes immutable projections; Swift 6 strict concurrency,
  no GCD; presentation-only formatting allowed, semantic labels arrive as
  Rust facts.

## Current state (entry gate)

- Plan 019 shipped: XCFramework, generated Swift bindings, decode layer,
  conformance suite, signing pipeline, `native/` proof harness.
- No app target exists. Repo layout for the app: extend `native/` (e.g.
  `native/TableRock/` Xcode project or SwiftPM-based app) — record the
  layout decision in the first checkpoint's evidence.

## Scope (checkpoints)

1. **App shell**: `App`/`WindowGroup`/`Commands`/`Settings` + restoration;
   toolbar with glass clusters (connection facts, run/cancel, save/preview/
   export placeholders); `NavigationSplitView` with glass sidebar; status
   bar with `sharedBackgroundVisibility(.hidden)` items. Appearance audit
   fixture: light/dark × Increase Contrast × Reduce Transparency
   screenshots recorded in evidence.
2. **Store + bridge client**: `@MainActor` presentation store; actor bridge
   client (poll `next_events`, decode pages off-main, publish immutable
   snapshots); connection list/editor screens (SwiftUI `List` sections +
   `Form` per connections.md "Both clients"), Keychain password source as
   the native-only addition (thin adapter returning transient bytes to the
   Rust request — never published in observable state).
3. **Catalog**: `NSOutlineView` representable over catalog snapshots; lazy
   expansion driving `RefreshCatalog` through the bridge; explicit
   loading/stale/error node states.
4. **Grid + editor + results**: `NSTableView` grid over decoded pages
   (resident-window model mirroring the TUI's; no per-cell FFI);
   `NSTextView`/TextKit editor with native find/IME; statement execution +
   streaming results + cancel; typed-distinction rendering; value
   inspector.
5. **Safety review**: the reviewed-operation sheet (statement list from the
   Rust preview facts; apply via review-token handle); safety modes
   enforced by Rust — Swift renders absence for ReadOnly.
6. **Accessibility tracer + conformance**: VoiceOver labels/roles on every
   custom/wrapped control, full keyboard path, focus restoration; bridge/
   native conformance run against the same engine fixtures as the TUI
   (plan 019 suite reused).

**Out of scope**: full parity (plan 021), import/export UI, multi-window
restoration completeness, engine-specific screens beyond the vertical
(Redis key browser lands in 021 unless trivially shared).

## Commands

`xcodebuild build/test` with strict concurrency; Instruments page/scroll
traces at measured result sizes (native-macos-path.md bridge gate);
signing/notarization pipeline from plan 019 for Release builds; Rust suites
unchanged-green.

## Done criteria

- [ ] Vertical slice: connect → catalog → query → stream → page → cancel → one reviewed safe operation on each applicable engine (recorded demo + tests)
- [x] Zero per-cell bridge calls (page snapshot decoded once off-main, then
      rendered by reusable `NSTableView` cells; evidence 408–409)
- [ ] Swift contains no SQL parsing/safety/mutation construction (code review checklist in evidence; grep for sql-building patterns)
- [ ] Glass rules hold: no glass on content surfaces; one cluster per region; accessibility degradation verified (screenshot matrix)
- [x] Strict-concurrency build clean; no `ObservableObject`, no GCD (evidence
      407–408; Swift 6 complete checking + warnings-as-errors)
- [ ] Instruments: native 10k-grid scroll/RSS recorded (evidence 505); UniFFI
      page-decode latency and retained-object attribution remain
- [ ] Conformance suite green through the app's bridge path
- [ ] Evidence + ROADMAP Phase 13 complete; `plans/README.md` updated

## STOP conditions

- Any capability tempts Swift-side database logic (e.g. editability
  inference) — STOP; the fact must come from Rust (contract addition if
  missing).
- Liquid Glass API surface differs from the doc's named APIs on the actual
  macOS 26 SDK — follow Apple's current guidance, record deviations in
  evidence; if a rule becomes unsatisfiable, STOP.
- Bridge performance misses the Instruments budgets — STOP (gate, not
  polish).

## Maintenance notes

- Plan 021 completes parity on this skeleton; keep store/projection
  boundaries screen-shaped so parity work is additive.
- Reviewer: MainActor hygiene, decode-off-main, immutability of published
  snapshots, accessibility completeness on wrapped AppKit views.
