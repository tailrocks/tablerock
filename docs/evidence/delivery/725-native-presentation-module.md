# Native Presentation module enforcement

Date: 2026-08-13

Checkpoint: P5 — enforce the Presentation module

## Decision

Production SwiftUI surfaces, AppKit adapters, presentation state, commands,
settings, and workflow coordination now compile in the independent
`TableRockPresentation` module. Its only target dependency is
`TableRockFeature`. `TableRockApp` is reduced to application/window
composition, platform ports, live-backend selection, and conditionally compiled
development backend support.

The public module seam is intentionally narrow: `ContentView`,
`WorkbenchPresentationStore`, `WorkbenchCommands`, `NativeSettingsView`, and
the development-only fixture surfaces needed by the app host. Generated UniFFI
types remain confined to TableRockBridge and the app composition root.

## Bounds and failure truth

- The move preserves presentation and backend behavior; no database contract
  changed.
- Presentation development support stays compile-time excluded from Release by
  the P6a condition.
- The scripted-backend scenario and lifetime tests remain app integration tests
  because their backend owner is app DevelopmentSupport. Fixture-configuration
  tests moved to `TableRockPresentationTests`, which does not import the app
  target.
- The older Row Continuum static neighbor map remains an explicit P6 debt; this
  checkpoint does not claim live Rust ownership for it.

## Compiler-enforced graph

```text
TableRockPresentation -> TableRockFeature
TableRockBridge       -> TableRockFeature + generated FFI
TableRockApp          -> TableRockPresentation + TableRockBridge + TableRockFeature
TableRockDesignLab    -> no dependencies
```

## Evidence

- XcodeGen produced a four-target production graph with Presentation depending
  only on Feature.
- `scripts/verify-native-source-ownership.sh` passed and now validates both
  SwiftPM and XcodeGen Presentation edges.
- `scripts/build-native-app.sh` independently compiled Feature, Bridge,
  Presentation, then the app with strict concurrency and warnings as errors.
- `swift package dump-package --package-path native` passed.
- `swift test --package-path native -c release` passed: 57 tests, 4 live-server
  skips, 0 failures.
- Focused Xcode suites passed: Feature 37/0, Presentation 3/0, App integration
  21/0.
- Release archive succeeded for arm64 and x86_64 with the four-target graph.
- `scripts/verify-native-query-tabs.sh` passed its structural/runtime gate after
  fixture surfaces moved under Presentation development support.
- `git diff --check` passed.

## Clean-room provenance

This checkpoint changes ownership and module boundaries only. TablePro public
user-visible organization informed the confirmed workbench direction; no
TablePro source, tests, comments, bundle internals, branding, assets, copy, or
proprietary fixtures were used.
