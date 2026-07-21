# 562 — Native XCTest and hosted checkpoint gate

Date: 2026-07-21

## Decision

Native bridge and feature tests use XCTest discovery instead of relying on
Swift Testing attributes. Every method has a `test` prefix, so Xcode and
SwiftPM report the exact executed count rather than accepting a compiled but
undiscovered suite. The process-global UniFFI lifecycle suite serializes each
test with one lock.

The pinned `Native Checkpoint` workflow is the authoritative macOS gate. It
runs on `macos-26`, selects stable Xcode 26.6, installs Rust 1.97.1, rejects
generated-binding drift, executes Swift tests, builds the universal static
XCFramework, builds the development app, verifies strict ad-hoc signing and
bundle identity, checks both XCFramework architectures, and archives the exact
structural outputs.

## Evidence

Run: [29801533506](https://github.com/tailrocks/tablerock/actions/runs/29801533506)

- Host: macOS 26.4, Xcode 26.6, Swift 6.3.3, Rust 1.97.1.
- XCTest: 21 executed, 0 failures. The report names all five suites:
  `AppConfigurationTests`, `AppDependenciesTests`, `BridgeLifecycleTests`,
  `PageV1BoundaryTests`, and `PageV1FixtureTests`.
- Generated UniFFI bindings: clean after regeneration.
- Universal `tablerock_ffiFFI.xcframework`: arm64 + x86_64, green.
- `TableRock.app`: built; `codesign --verify --deep --strict` and exact bundle
  identifier passed.
- Artifact `native-checkpoint-f660cac6a97a9ff6bb73dbb2cd6bac41ab936a0e`:
  ID `8484208824`, 12 files, SHA-256
  `f8fac658756fa3b5770356f4dcb709fb960374a8dd5e9869eb531054d50f49d8`.
- Local CLT 26.6 exposes Swift 6.3.3 but neither XCTest nor Swift Testing to
  `swift test`; the hosted full-Xcode result is therefore the platform proof.

This closes the missing every-push/scheduled Swift regression and unsigned
structural package gate. It does not claim checkpoint 14 XCUITest plans,
checkpoint 15's canonical Xcode shipping app, Developer ID signing,
notarization, or stapling.

## Provenance

External concepts: XCTest method discovery; pinned Xcode runner validation;
artifact retention.

Public sources: <https://developer.apple.com/documentation/xctest>,
<https://docs.github.com/actions/using-github-hosted-runners/about-github-hosted-runners>,
<https://docs.github.com/actions/using-workflows/storing-workflow-data-as-artifacts>.

Implementation source: TableRock-owned tests and workflow.

TablePro influence: none; this checkpoint changes test and delivery
infrastructure, not product workflow or visual expression.

Copied code, tests, assets, strings, identifiers, colors, geometry, layout
measurements, or key bindings: none.
