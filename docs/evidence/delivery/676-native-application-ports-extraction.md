# Native application ports extraction

Date: 2026-08-12

Checkpoint: P2a — extract platform capability adapters

## Decision

macOS file panels, pasteboard, Keychain, application-support path resolution,
and test-confined file panels now live in
`native/Sources/TableRockApp/ApplicationPorts.swift`. These are application
composition adapters for bridge-neutral `TableRockFeature` ports. They no
longer share a compilation unit with backends, presentation state, SwiftUI
surfaces, or AppKit catalog/grid/editor adapters.

The extraction is behavior-preserving. Only file-scoped `private` visibility
was widened to module-internal visibility where `NativeApplicationModel`
constructs the adapters. No API is public outside the application target.

## Bounds and failure truth

- Keychain, file-panel confinement, pasteboard representation, and security-
  scoped file behavior did not change.
- Rust/UniFFI, database, safety, persistence, and presentation behavior did not
  change.
- `TableRockApp.swift` remains 12,100 lines and still owns other mixed
  responsibilities. This is one forward structural checkpoint, not completion
  of P2.
- Security and Uniform Type Identifier imports moved with their sole owners.

## Evidence

- `scripts/verify-native-source-ownership.sh` passed.
- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- XcodeGen regenerated the project with `ApplicationPorts.swift` in the app
  target.
- `TableRockAppTests`: 21 passed, zero failures. This includes
  `testTestFilePanelsConfineOpenAndSavePathsToIsolatedRoot`.
- `git diff --check` passed.

## Remaining work

Continue P2 with application runtime, entry/window hosting, commands, live
backend, and bridge conversion ownership. Keep each extraction buildable before
introducing the Presentation module.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
