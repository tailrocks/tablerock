# Native PostgreSQL-sheet extraction

Date: 2026-08-13

Checkpoint: P4w — extract PostgreSQL administration presentation

## Decision

PostgreSQL role, relationship, activity/signal, and supervised-tool sheets now
live in `PostgresSheets.swift`.

## Bounds and failure truth

- Role search and comparison, relationship selection, bounded activity facts,
  signal confirmation, maintenance progress/cancellation, backup review, and
  outcome/error presentation moved unchanged.
- PostgreSQL semantics, permissions, signals, maintenance operation lifetime,
  review authority, and failure truth remain below presentation.
- No backend, fixture, generated FFI, AppKit, or Design Lab dependency entered
  the PostgreSQL file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures, including supervised tool and
  role/relationship scenarios.
- `scripts/verify-native-maintenance.sh` passed its PostgreSQL maintenance
  lifecycle gate.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 2,806 to 2,331 lines.

## Remaining work

Extract result/inspector and environment chrome. Then enforce the Presentation
module and remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
