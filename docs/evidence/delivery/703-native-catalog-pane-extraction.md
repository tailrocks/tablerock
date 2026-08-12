# Native catalog-pane extraction

Date: 2026-08-13

Checkpoint: P4f — extract live catalog navigation pane

## Decision

The connected-session catalog pane now lives in
`ConnectionsCatalogPane.swift`. Native refresh controls, progress, stale-content
preservation, failure/empty states, outline expansion, selection, and object
opening form one catalog-owned surface.

## Bounds and failure truth

- Refresh disabling, loading status, catalog outline bindings, async expand/open
  actions, and native unavailable states moved unchanged.
- The pane consumes derived presentation state and delegates catalog operations
  to the store. Database catalog behavior remains behind the backend.
- No fixture symbol, generated FFI type, backend import, custom glass, or Design
  Lab dependency entered the pane file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 6,132 to 6,065 lines.

## Remaining work

Extract the workbench shell/surfaces and narrow AppKit adapters. Then enforce
the Presentation module and remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
