# Native history and saved-query workflow extraction

Date: 2026-08-13

Checkpoint: P3h — extract history and saved-query workflows

## Decision

Query-history loading, retention, restoration, and saved-query loading, save,
restore, and deletion now live in `WorkbenchPresentationStore+Library.swift`.
The production presentation store remains the single state owner; this change
only moves cohesive workflow methods out of the primary declaration.

## Bounds and failure truth

- Backend calls, generation-based stale-response rejection, error messages, and
  editor restoration behavior moved unchanged.
- History and saved-query loading/error/generation state became
  module-internal for same-type cross-file access. No public API was added.
- Database and persistence policy remain behind `WorkbenchBackend`; no I/O
  entered SwiftUI update or rendering paths.
- No fixture symbol, generated FFI type, or Design Lab dependency entered the
  new extension.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- `scripts/verify-native-history.sh` passed structural and runtime proof.
- `scripts/verify-native-saved-queries.sh` passed structural and runtime proof.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.

## Remaining work

Extract tabs/navigation, transfer, and administration workflows. Then enforce
the Presentation module and remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
