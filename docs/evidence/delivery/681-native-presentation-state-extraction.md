# Native presentation state extraction

Date: 2026-08-12

Checkpoint: P2f — extract fixture-free presentation state

## Decision

Window presentation values for catalog refreshes, profile dialogs, connection
URL review, quick switching, selected cells, query tabs, and object tabs now
live in `native/Sources/TableRockApp/PresentationState.swift`.

The extracted types project `TableRockFeature` values into mutable native UI
state. They own no bridge object, database operation, persistence, safety
policy, fixture selection, or rendered view. Catalog key and descendant helpers
moved with their state because they encode presentation identity and expansion
behavior.

## Bounds and failure truth

- Stored properties, initial values, identity, observation, and actor isolation
  did not change.
- Catalog descendant traversal and key formatting remain byte-for-byte
  equivalent.
- The two catalog helpers became module-internal because `BridgeModel` consumes
  them from another file. No public API was added.
- Existing fixture and scripted-backend debt did not move or spread.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `scripts/verify-native-profile-editor.sh` passed structural and runtime proof.
- `scripts/verify-native-profile-groups.sh` passed structural and runtime proof.
- `scripts/verify-native-query-tabs.sh` passed structural and runtime proof.
- `scripts/verify-native-object-tabs.sh` passed structural and runtime proof.
- `scripts/verify-native-accessibility.sh` passed structural and runtime proof.
- `TableRockAppTests`: 21 passed, zero failures.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target with `PresentationState.swift`.
- `git diff --check` passed.

## Remaining work

Extract live bridge and model ownership without moving frozen development
fixture debt, then establish the compiler-enforced Presentation/Feature/Bridge
dependency graph.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
