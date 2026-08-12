# Native safety workflow extraction

Date: 2026-08-13

Checkpoint: P3n — extract DDL and table-operation safety workflows

## Decision

DDL review/apply/revoke and table-operation review, exact confirmation,
supervised status, result refresh, and cleanup now live in
`WorkbenchPresentationStore+Safety.swift`.

The presentation store retains dialog state only. Frozen reviews, tokens,
confirmation authority, operation status, and database mutation remain behind
`WorkbenchBackend` and therefore below presentation.

## Bounds and failure truth

- Review staging, token consumption/revocation, exact-name confirmation,
  progress polling, structure/result refresh, tab invalidation, and error text
  moved unchanged.
- DDL/table operation identifiers and applying flags became module-internal for
  same-type cross-file access. No public API was added.
- No mutation logic, review authority, fixture symbol, generated FFI type, or
  Design Lab dependency entered the safety extension.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- Structure-change frozen-review/token-consumption behavior passed.
- Table-operation frozen-target/exact-confirmation behavior passed.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.

## Remaining work

Complete the store initialization/fixture isolation checkpoint, then enforce the
Presentation module and remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
