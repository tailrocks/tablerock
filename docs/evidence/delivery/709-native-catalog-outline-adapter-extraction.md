# Native catalog-outline adapter extraction

Date: 2026-08-13

Checkpoint: P4l — isolate the AppKit catalog outline

## Decision

The catalog `NSOutlineView` adapter, node projection, expansion restoration,
selection binding, and double-click routing now live in
`CatalogOutline.swift`.

This creates one narrow AppKit ownership boundary for hierarchical catalog
rendering. SwiftUI still owns pane composition and the presentation store still
owns navigation and refresh state.

## Bounds and failure truth

- Default expansion, lazy-expand callbacks, selection restoration, stale and
  loading rows, accessibility identifiers, and open routing moved unchanged.
- No backend, fixture, generated FFI, or Design Lab dependency entered the
  adapter.
- The adapter depends only on AppKit, SwiftUI, feature catalog records, and
  module-owned presentation state.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- `scripts/verify-native-structure.sh` passed its shared TUI/native PostgreSQL
  structure runtime gate.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 5,267 to 5,017 lines.

## Remaining work

Extract the result-grid adapter, then enforce the Presentation module and
remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
