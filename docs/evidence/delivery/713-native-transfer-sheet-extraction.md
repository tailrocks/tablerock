# Native transfer-sheet extraction

Date: 2026-08-13

Checkpoint: P4p — extract import and export presentation

## Decision

Full-result export and reviewed CSV-import sheets now live in
`TransferSheets.swift`. Shared change-review composition became
module-internal so transfer and safety surfaces can use one visual contract.

## Bounds and failure truth

- Bounded export progress, cancellation, atomic destination presentation, CSV
  mapping, formula-like literal warnings, frozen review, reviewed apply,
  partial errors, error copying, and dismissal guards moved unchanged.
- Rust still owns replay, paging, review authority, transaction behavior, and
  final outcomes.
- No backend, fixture, generated FFI, AppKit, or Design Lab dependency entered
  the transfer file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- `scripts/verify-native-csv-import.sh` passed CSV preview, reviewed apply, and
  live PostgreSQL transaction gates.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 4,355 to 4,105 lines.

## Remaining work

Extract engine, review, inspector, tab, toolbar, and settings surfaces. Then
enforce the Presentation module and remove Release fixture/scripted-backend
membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
