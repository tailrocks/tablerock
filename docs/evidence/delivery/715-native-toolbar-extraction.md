# Native toolbar extraction

Date: 2026-08-13

Checkpoint: P4r — extract functional workbench toolbar

## Decision

Connection, environment/safety, file, refresh, run, and cancel toolbar content
now lives in `WorkbenchToolbar.swift`.

## Bounds and failure truth

- System placements, identifiers, labels, glass-prominent run action, health
  and reconnect controls, and all capability-derived disabled states moved
  unchanged.
- Toolbar actions still send intents to the single presentation store; no
  bridge work occurs during rendering.
- No backend, fixture, generated FFI, AppKit, or Design Lab dependency entered
  the toolbar file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- `scripts/verify-native-query-tabs.sh` passed its structural and runtime gate.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 3,960 to 3,807 lines.

## Remaining work

Extract object controls, review, PostgreSQL, inspector, tabs, and settings.
Then enforce the Presentation module and remove Release fixture/scripted-
backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
