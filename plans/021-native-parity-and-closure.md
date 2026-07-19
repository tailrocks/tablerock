# Plan 021: Native workflow parity, release evidence, and parity closure (Phases 14–15)

> **Executor instructions**: Work-package plan covering the final two roadmap
> phases. Authority: delivery-plan.md "Phase 14"/"Phase 15", ROADMAP 14–15,
> parity-ledger "Native macOS parity" + "Closure rule",
> `docs/product/native-macos.md`. STOP conditions binding. Update
> `plans/README.md` when done.
>
> **Drift check (run first)**: plan 020 DONE.

## Status

- **IN PROGRESS (2026-07-19): native connection workflows (evidence 512–518), history/retention (519–520), saved queries (521), SQL files (522), typed intent bridge (523), query tabs (524), read-only preview/pinned object tabs (525), multi-window restoration (526), environment/safety projection (527), typed value inspection (528), shared Rust/native multi-format copy (529), native atomic loaded-result export (530), shared bounded CSV import foundation (531), native reviewed CSV import (532), PostgreSQL/ClickHouse structure (533–534), bounded Redis key catalog projection (535), native Redis key object views (536), native Redis sampled overview (537), SwiftPM bridge regression foundation/boundary expansion (538–539), and deterministic cross-engine PageV1 fixtures (540) landed; testing-system checkpoint 11 remains partial and checkpoints 12–16 remain unimplemented; broader import types/engines, full streaming export, advanced object state, and remaining screens continue**
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

## Required testing system

Testing follows the lowest reliable boundary. Swift UI automation never
substitutes for Rust database semantics, and source greps/log markers never
substitute for user-operable application tests. Existing proof executables and
`verify-native-*.sh` scripts remain useful architecture/performance gates while
the durable suites below become authoritative.

### Test layers and cadence

| Layer | Required proof | Backend | Main-branch cadence |
|---|---|---|---|
| Rust contracts | Domain rules, bounds, page bytes, revisions, event ordering, cancellation truth, redaction, mutation authority | Deterministic Rust fakes | Every commit |
| Rust client behavior | Root TEA reducer/effects/rendering plus real CLI process keyboard, focus, paging, dirty-close, cancel, and shutdown workflows | Scripted ports for deterministic tests; PTY harness for the built TUI | Every commit smoke; full matrix nightly |
| Rust real servers | PostgreSQL, ClickHouse, and Redis protocol/driver behavior and cross-engine bridge conformance | Pinned containers/dedicated servers | Every pushed checkpoint; full supported-version matrix nightly |
| Swift bridge | Generated UniFFI ownership, lifecycle, typed errors, page decoding, cancellation handles, shutdown, redaction | Real Rust library; deterministic drivers except named live cases | Every macOS checkpoint |
| Swift feature model | Presentation state transitions for profiles, sessions, tabs, pages, errors, cancel races, restoration, and multi-window ownership | Injected scripted `WorkbenchBackend` | Every macOS checkpoint |
| XCUITest | Real clicks, keyboard, menus, focus, accessibility, windows, restoration, and lifecycle | Scripted backend for smoke; named live-server cases nightly | Smoke every macOS checkpoint; full nightly |
| Packaged application | XCFramework linkage, architectures, rpaths, hardened runtime, launch, migration, update, crash recovery, and uninstall residue | Exact Release `.app` | Nightly unsigned structural gate; signed/notarized release gate |

### Dependency-ordered checkpoints

11. **SwiftPM regression foundation**
    - Add importable `TableRockFeature` and separate
      `TableRockBridgeTests`/`TableRockFeatureTests` test targets. SwiftPM
      target resources own committed versioned fixtures.
    - Convert every `BridgeProof`/`BehaviorProof` assertion into an isolated,
      named test with independent setup/teardown; proof executables may remain
      only until equivalent tests are green, then remove them.
    - Add PageV1 golden/hostile fixtures for all engines, NULL/empty/binary/
      structured/invalid/truncated values, every size bound, bad offsets,
      overflow, unsupported versions, and repeated decode ownership.
    - Bridge lifecycle coverage: create/ensure/destroy idempotence, panic
      containment, malformed IDs, unreachable-server redaction, calls after
      shutdown, repeated create/free, and bounded shutdown with active work.

12. **Injectable native presentation boundary**
    - Define an application-owned `WorkbenchBackend: Sendable` and
      application-owned immutable DTOs. `LiveWorkbenchBackend` is the sole
      generated-UniFFI adapter; `BridgeModel` depends only on the protocol.
    - Inject `AppConfiguration`, `AppPaths`, clock/UUID, Keychain, file-panel,
      and pasteboard ports. Production defaults remain unchanged.
    - Test mode is parsed once at startup. Every test receives a unique
      temporary data root and isolated persistence/Keychain namespace; tests
      must never touch the operator's Application Support, profiles, history,
      restoration, files, or pasteboard unless that capability is the test.
    - `ScriptedWorkbenchBackend` scenarios cover success, connection/auth
      failure, slow-until-cancelled, stale result revision, stale event,
      cursor resync, mismatched next-page columns, history failure after page,
      running/dirty tab close, restoration corruption, multi-window ownership,
      and model deallocation during work.

13. **Stable native automation surface**
    - Add stable accessibility identifiers independent of visible/localized
      text for at least `window.workbench`, `sidebar.profiles`, `profile.add`,
      `profile.editor.name`, `profile.editor.save`, `catalog.outline`,
      `query.editor`, `query.run`, `query.cancel`, `query.status`,
      `results.grid`, `results.next-page`, and stable-ID query tabs.
    - AppKit wrappers set both human-readable labels and stable identifiers.
      Durable entities use their profile/tab/catalog IDs, never row positions.
    - Keep the existing structural accessibility scripts as policy gates;
      XCUITest becomes the behavioral authority for actual clicks, typing,
      focus, window lifecycle, menus, and VoiceOver-visible state.

14. **Canonical Xcode application and test plans**
    - Add one Xcode application project/scheme consuming the importable Swift
      targets and an XCUITest target. `PullRequest.xctestplan` is named
      `Checkpoint.xctestplan` under trunk-only delivery; also add Nightly and
      Release plans.
    - Checkpoint plan: all deterministic feature/decoder/lifecycle tests,
      5–10 scripted UI workflows, and canonical Release build/launch smoke.
      First automated workflow is slow query → click Cancel → observe honest
      terminal state; cancellation of a Swift task alone never claims Rust
      cancellation.
    - Nightly: full UI suite, one live PostgreSQL app-process query/cancel,
      all three live bridge engines, multi-window/restoration, IME/marked text,
      accessibility, bridge create/free stress, Time Profiler, Allocations,
      Leaks, and RSS gates.

15. **Exact shipping-artifact gate**
    - Make the Xcode Release application consuming the universal static
      XCFramework the single shipping path. Development dylib/SwiftPM and
      direct-`swiftc` paths remain explicitly separate development gates and
      may not support a release claim.
    - Build: Rust static libraries → universal XCFramework → Release Xcode
      app → embedded-code signing → hardened runtime → notarization → stapling.
    - Verify architectures, absence of absolute development-library paths,
      strict codesign, Gatekeeper, first launch/network/persistence, migration
      from the previous shipped schema, update, crash/relaunch restoration,
      clean-user/VM behavior, and documented uninstall residue.

16. **CI and evidence artifacts**
    - Trunk-only conflict resolution: no pull-request workflow is introduced.
      Fast deterministic Rust + native checkpoint gates run on every push to
      `main`; nightly and release workflows run on schedule/manual dispatch.
    - macOS jobs use an Xcode/macOS SDK matching the deployment target; live
      container matrices use a self-hosted Mac or dedicated servers when the
      hosted runner cannot provide them.
    - Always archive `.xcresult`, stdout/stderr, crash reports, failure
      screenshots, generated-binding diff, app metadata, checksums, and
      Instruments summaries. Artifacts and test-plan membership are linked
      from evidence; a green manifest without coverage inspection is not proof.

### Required workflow cases

- Rust TUI and native feature-model scripts express the same shared workflows
  and compare Rust outcomes: open/connect, catalog expansion, query/command,
  first/next page, cancel races, history, review/apply/discard, reconnect,
  restoration intent, disconnect, and shutdown.
- Native XCUITest operates controls for profile creation, connection, catalog
  expansion, run/cancel, result selection, paging, dirty/running tab close,
  multi-window restoration, and file import/export. At least one nightly case
  uses the live Rust backend; scripted tests never claim database semantics.
- Test harnesses prove cleanup after success, failure, cancellation, timeout,
  and test-process termination. No test reads or mutates real user data.

## Commands

Every checkpoint runs the applicable plans 018–020 gates plus:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo build -p tablerock-ffi --release
./scripts/generate-swift-bindings.sh
git diff --exit-code -- native/Generated native/Sources/TableRockBridge/tablerock_ffi.swift
(cd native && DYLD_LIBRARY_PATH=../target/release swift test -c release)
xcodebuild test -project native/App/TableRock.xcodeproj -scheme TableRock \
  -testPlan Checkpoint -destination 'platform=macOS' \
  -resultBundlePath artifacts/TableRock-Checkpoint.xcresult
```

Nightly adds all real-server suites, full XCUITest/IME/accessibility,
Instruments, and both supported architectures. Release adds XCFramework app
build plus `codesign --verify --deep --strict`, Gatekeeper assessment,
notarization submission, stapler validation, and clean-user/VM audits.
Evidence records exact resolved commands/tool versions; the generated-binding
diff must be empty after regeneration.

## Done criteria

- [ ] Every product screen exists natively per its "Both clients" row
- [ ] Workflow-equivalence suite green (same Rust outcomes both clients)
- [ ] Rust TUI process tests and deterministic TEA/effect suites green
- [ ] Swift bridge and injected feature-model test targets green
- [ ] Checkpoint/Nightly/Release Xcode test plans exist and required cases pass
- [ ] Every native test uses an isolated temporary root/capability namespace
- [ ] Stable accessibility identifiers cover all required automation surfaces
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
