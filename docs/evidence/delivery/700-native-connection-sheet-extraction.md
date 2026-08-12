# Native connection-sheet extraction

Date: 2026-08-13

Checkpoint: P4c — extract connection auxiliary sheets

## Decision

Transient password entry, profile-group editing, connection-URL review,
external-URL confirmation, quick switching, and explain-plan presentation now
live in `ConnectionSheets.swift`.

These are bounded navigation/dialog surfaces. They consume presentation drafts
or the environment-owned store and delegate work to existing workflow methods.

## Bounds and failure truth

- Secret clearing, async dismissal guards, URL review-before-action, external
  action confirmation, quick-switch keyboard behavior, explain copy, native
  toolbar placement, accessibility identifiers, and sizing moved unchanged.
- No connection parsing, credential persistence, query execution, or external
  URL authority entered SwiftUI views.
- No fixture symbol, generated FFI type, backend import, AppKit adapter, or
  Design Lab dependency entered the sheet file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- Profile-group structural/runtime gate passed.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 6,754 to 6,469 lines.

## Remaining work

Continue extracting workbench surfaces and narrow AppKit adapters. Then enforce
the Presentation module and remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
