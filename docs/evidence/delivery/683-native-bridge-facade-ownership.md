# Native Bridge facade ownership

Date: 2026-08-12

Checkpoint: P2h — move live facade and conversions into TableRockBridge

## Decision

`TableRockBridge` now owns both `LiveWorkbenchBackend.swift` and
`WorkbenchBridgeConversions.swift`. The module depends on `TableRockFeature`
and exposes one bridge-neutral factory:
`makeLiveWorkbenchBackend(persistencePath:) -> any WorkbenchBackend`.

The concrete actor remains private. Generated UniFFI types and conversion
properties remain module-internal, so presentation and application code cannot
construct or translate FFI records. The app factory still chooses live versus
scripted configuration but receives only the stable Feature protocol.

## Dependency graph

- `TableRockFeature` has no production-module dependency.
- `TableRockBridge` depends on `TableRockFeature` and `tablerock_ffiFFI`.
- `TableRockApp` depends on Feature and Bridge while production presentation is
  still being extracted.
- Design Lab remains dependency-free and absent from every production edge.

SwiftPM, XcodeGen, and direct `swiftc` builds now encode the same ordering. The
direct build compiles Feature first, Bridge second, and the application last.
The ownership gate rejects Bridge dependency drift in both SwiftPM and XcodeGen
specifications.

## Bounds and failure truth

- Backend method bodies, call order, event cursor, page decoding, cancellation,
  conversion values, and errors did not change.
- Generated bindings were regenerated and produced no tracked diff.
- No generated FFI type was added to the public factory surface.
- Scripted backend and fixture debt remain frozen in their existing files for
  the later development-support isolation checkpoint.

## Evidence

- `scripts/generate-swift-bindings.sh` passed with no generated diff.
- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors using the new Feature-before-Bridge graph.
- `swift test --package-path native -c release` built the production app and
  passed 56 XCTest cases with four configured live-server skips plus 24 Swift
  Testing cases with zero failures.
- Xcode checkpoint tests passed: 36 Feature, 20 Bridge with four configured
  live-server skips, and 21 App tests; zero failures.
- `scripts/verify-native-source-ownership.sh` passed and now enforces the Bridge
  dependency edge.
- XcodeGen regenerated the project with Bridge depending on Feature.
- `bash -n` and `git diff --check` passed.

## Remaining work

Split the window presentation store and view ownership. Later isolate scripted
backend and Release fixture injection without widening the production graph.

## Clean-room provenance

This checkpoint changes integration ownership only. TablePro public
user-visible organization informed the confirmed workbench direction; no
TablePro source, tests, comments, bundle internals, branding, assets, copy, or
proprietary fixtures were used.
