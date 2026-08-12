# Native settings extraction

Date: 2026-08-13

Checkpoint: P4s — isolate app-integrated settings

## Decision

Local-storage, telemetry, and safe support-bundle settings now live in
`NativeSettingsView.swift` beside the application integration boundary they
use.

## Bounds and failure truth

- Save-panel routing, extension normalization, security-scoped access,
  redacted backend export, success fact, and closed failure message moved
  unchanged.
- Settings remain in the app target because they compose application-owned
  backend and platform ports; they are not presentation-module candidates.
- No fixture, generated FFI, AppKit adapter, or Design Lab dependency entered
  the settings file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 3,807 to 3,753 lines.

## Remaining work

Extract object controls, review, PostgreSQL, inspector, and tabs. Then enforce
the Presentation module and remove Release fixture/scripted-backend
membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
