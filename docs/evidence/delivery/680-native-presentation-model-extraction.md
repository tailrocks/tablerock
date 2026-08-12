# Native presentation model extraction

Date: 2026-08-12

Checkpoint: P2e — extract presentation-only helpers and editor drafts

## Decision

Connection-status formatting, transient-data zeroization, hexadecimal
presentation encoding, and the mutable startup-action and profile-editor forms
now live in `native/Sources/TableRockApp/PresentationModels.swift`.

These types adapt immutable `TableRockFeature` values for native editing. They
do not own bridge objects, database operations, safety policy, persistence, or
fixture selection. Keeping the conversion back to feature values beside each
form gives presentation mutation one explicit boundary.

## Bounds and failure truth

- Form fields, default values, conversion order, labels, and save/test behavior
  did not change.
- Transient secret zeroization remains byte-for-byte identical and still runs
  at the same call sites.
- `Data.hexEncodedString()` became module-internal because its consumers remain
  in the app target after the extraction. No public API was added.
- Deterministic fixture code remains in its frozen source path. This checkpoint
  introduced no fixture or scripted-backend debt.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `scripts/verify-native-profile-editor.sh` passed structural and runtime proof.
- `scripts/verify-native-profile-groups.sh` passed structural and runtime proof.
- `TableRockAppTests`: 21 passed, zero failures.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target with `PresentationModels.swift`.
- `git diff --check` passed.

Two profile verifier attempts launched concurrently and collided in the shared
direct-build directory (`tablerock_ffi.o` and `PageV1.o`). Sequential reruns
passed. The failure was harness resource contention, not product behavior.

## Remaining work

Continue extracting fixture-free presentation state and backend ownership, then
establish the compiler-enforced Presentation/Feature/Bridge dependency graph.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
