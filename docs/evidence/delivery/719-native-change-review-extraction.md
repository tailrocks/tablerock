# Native change-review extraction

Date: 2026-08-13

Checkpoint: P4v — extract reviewed mutation presentation

## Decision

The shared opaque Change Review plane plus probe, DDL, and table-operation
sheets now live in `ChangeReviewSheets.swift`.

## Bounds and failure truth

- Kind-first review facts, frozen preview metadata, expiry, production halo,
  exact destructive confirmation, apply/discard/cancel state, one-use authority
  messaging, and ambiguous/failure outcomes moved unchanged.
- Rust still owns mutation plans, review tokens, expiry, authority consumption,
  execution, rollback truth, and safety decisions.
- No backend, fixture, generated FFI, AppKit, or Design Lab dependency entered
  the review file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures, including DDL and table-
  operation review scenarios.
- `scripts/verify-native-maintenance.sh` passed its PostgreSQL maintenance
  lifecycle gate.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 3,214 to 2,806 lines.

## Remaining work

Extract PostgreSQL, result/inspector, and environment chrome. Then enforce the
Presentation module and remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
