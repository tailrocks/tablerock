# Native live backend extraction

Date: 2026-08-12

Checkpoint: P2g — extract the live backend facade from the app monolith

## Decision

The actor that owns the generated synchronous UniFFI bridge, event cursor,
page decoding, and conversion calls now lives in
`native/Sources/TableRockApp/LiveWorkbenchBackend.swift`.

This is an intermediate compiler-checked boundary. The actor remains in the app
target until it moves together with its bridge conversions into
`TableRockBridge`; moving both together avoids exposing generated FFI records
through public conversion APIs.

## Bounds and failure truth

- Every protocol method body, call order, event-loop bound, detached decoding
  task, cancellation path, and error path moved unchanged.
- Actor visibility changed from file-private to module-internal so the existing
  app factory can construct it from another file. No public API was added.
- Backend selection remains owned by the existing application factory.
- Scripted backend and fixture code remain in their frozen source path. No
  fixture debt moved or spread.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `swift test --package-path native -c release` built the production app and
  passed 56 XCTest cases with four configured live-server skips plus 24 Swift
  Testing cases with zero failures.
- `TableRockAppTests`: 21 passed, zero failures.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target with `LiveWorkbenchBackend.swift`.
- `git diff --check` passed.

## Remaining work

Move this actor and `WorkbenchBridgeConversions.swift` into `TableRockBridge`,
make Bridge depend on Feature, and update direct, SwiftPM, and Xcode graphs in
one compiler-enforced checkpoint.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
