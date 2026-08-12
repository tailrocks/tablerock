# Native result-grid adapter extraction

Date: 2026-08-13

Checkpoint: P4m — isolate the AppKit result grid

## Decision

The result `NSTableView` adapter, reusable cells, column projection, selection
routing, accessibility projection, and bounded performance-scroll driver now
live in `CatalogGrid.swift`.

The adapter receives performance-proof activation as an explicit Boolean from
app composition. It no longer reads process environment or fixture state
itself.

## Bounds and failure truth

- Column order and resizing, selected-row restoration, typed-cell projection,
  null and empty differentiation, accessibility values, and activation routing
  moved unchanged.
- Existing performance-scroll activation remains projected by
  `NativeWorkbenchFixtureConfiguration`; empty production environments project
  `false`.
- No backend, fixture symbol, generated FFI, or Design Lab dependency entered
  the adapter.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures, including empty and populated
  fixture-environment projection.
- `scripts/verify-native-performance.sh` completed the 10,000-row bounded
  scroll: 1.528989 seconds, 155,840 KiB maximum RSS, and a 17,924,096-byte Time
  Profiler trace.
- `scripts/verify-native-source-ownership.sh` passed after fixture control was
  injected from composition.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 5,017 to 4,744 lines.

## Remaining work

Extract remaining SwiftUI surfaces, then enforce the Presentation module and
remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
