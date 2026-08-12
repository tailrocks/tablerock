# Native production migration plan

Date: 2026-08-12

Checkpoint: production architecture package after Native Workbench confirmation

## Decision

Production migration will eliminate the native monolith through compiler-
enforced ownership, not a cosmetic file split. A new `TableRockPresentation`
module will depend only on bridge-neutral `TableRockFeature` contracts.
`TableRockApp` becomes the composition and platform-integration root;
`TableRockBridge` owns UniFFI translation and the live backend facade. Design
Lab remains dependency-free evidence and never becomes a production input.

The implementation sequence first makes existing gates multi-file aware, then
extracts integration, state, SwiftUI, and AppKit responsibilities without
behavior change. Only after those boundaries pass does visual migration begin.

## Bounds and failure truth

- Rust retains database semantics, persistence, safety, redaction, credentials,
  and operation truth.
- SwiftUI remains primary. AppKit remains limited to the native catalog, result
  table, and SQL editor boundaries.
- Release output must not contain scripted backend or deterministic fixture
  routes.
- Liquid Glass is functional chrome only; data/editor/inspector/review content
  remains opaque.
- The operator-rejected Sketch has no authority and is not restored.
- Developer ID signing/notarization remains an existing credential-gated proof;
  this checkpoint does not claim it.
- Structural recovery is forward-only by repair or a new revert commit.

## Evidence

- Explicit operator approval: evidence 673.
- Direct strict Swift build passed before planning.
- Feature, Bridge, and App Xcode suites passed: 77 tests, four configured live
  skips, zero failures.
- The local app-test host required `ENABLE_HARDENED_RUNTIME=NO` with ad-hoc
  signing because host and test bundle Team IDs differ. Shipping settings and
  canonical CI remain hardened.
- `./scripts/verify-native-design-lab.sh` passed after the package was written:
  dependency isolation, independent Swift/Xcode builds, 10 model tests, and 5
  real-window UI tests all passed.
- Repository inspection found 17 verifiers coupled to
  `TableRockApp.swift`; checkpoint P1 removes that structural coupling before
  extraction.
- Full ownership, prerequisites, drift checks, implementation steps,
  verification commands, done criteria, stop conditions, and recovery routes
  are recorded in
  [`production-migration-plan.md`](../../design/native-macos-2026/production-migration-plan.md).

## Remaining work

Execute P1–P15 in order. Commit and push every coherent green checkpoint. Do
not begin a later ownership or visual boundary while an earlier checkpoint is
red. Final completion still requires production real-window visual and
accessibility evidence plus every current repository readiness gate.

## Clean-room provenance

TablePro public user-visible organization influenced broad direction only.
Apple platform behavior and the confirmed TableRock design govern production.
No TablePro source, tests, comments, bundle internals, branding, assets, copy,
or proprietary fixtures were used.
