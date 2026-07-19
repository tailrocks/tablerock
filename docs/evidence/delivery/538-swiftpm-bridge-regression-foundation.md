# SwiftPM bridge regression foundation

Date: 2026-07-19

## Contract

The native Swift package now owns a real `TableRockBridgeTests` test target.
Five proof-executable assertions became isolated Swift Testing cases covering
panic containment/process usability, idempotent runtime lifecycle, typed and
redacted unreachable-server errors, bad PageV1 magic, and pre-body oversized
arena rejection.

The obsolete `BridgeProof` executable was removed only after all its assertions
passed in the durable suite. Tests link the real generated UniFFI bridge and
Rust release library; they do not launch the application or infer database
semantics from Swift.

## Evidence

- Full Xcode 26.6 / Swift 6.3.3 host.
- `cargo build -p tablerock-ffi --release`: pass.
- `DYLD_LIBRARY_PATH=../target/release swift test -c release`: five tests in
  two suites pass.
- The first test run exposed path-dependent generated-type `Identifiable`
  conformances: SwiftPM rejected `@retroactive`, while direct module builds
  required it. Removing those conformances and using explicit stable IDs/app
  sheet state made both build paths clean.
- Secret sentinel is absent from both the bridge rejection message and the
  thrown error description.
- Direct native app build and accessibility runtime gate: pass.

## Remaining boundary

Checkpoint 11 remains partial: versioned cross-engine golden fixtures, full
hostile PageV1 bodies, cancellation/shutdown stress, repeated ownership tests,
and `BehaviorProof` conversion remain. Feature-model injection, XCUITest, test
plans, CI, and packaged-app gates are checkpoints 12–16.

## Provenance

This testing structure follows current Swift Package Manager target/testTarget
and resource guidance plus the operator-supplied Apple testing model. TablePro
is only a broad product-workflow reference; no source, tests, text, screenshots,
layouts, measurements, colors, assets, or key bindings were copied or
translated.
