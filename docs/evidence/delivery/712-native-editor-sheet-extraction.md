# Native editor-sheet extraction

Date: 2026-08-13

Checkpoint: P4o — extract editor workflow sheets

## Decision

Find/replace and typed query-parameter sheets now live in
`EditorSheets.swift`. The sheets remain pure SwiftUI projections over the
single presentation store.

## Bounds and failure truth

- Find modes, selection scope, traversal reset, replace actions, query
  parameter kinds, cancellation, run enablement, and dismissal behavior moved
  unchanged.
- Query parameter values still cross the Rust boundary separately from SQL
  text through existing store intents.
- No backend, fixture, generated FFI, AppKit, or Design Lab dependency entered
  the sheet file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- `scripts/verify-native-query-tabs.sh` passed its structural and runtime gate.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 4,496 to 4,355 lines.

## Remaining work

Extract transfer, engine, review, inspector, tab, toolbar, and settings
surfaces. Then enforce the Presentation module and remove Release
fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
