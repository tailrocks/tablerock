# Native restoration workflow extraction

Date: 2026-08-13

Checkpoint: P3f — extract window and session restoration

## Decision

Window-intent lookup, session-intent application, invalid-restoration handling,
and volatile tab cleanup now live in
`WorkbenchPresentationStore+Restoration.swift`.

The extension restores only feature-level intent values. It neither persists
directly nor receives generated bridge records. Connection replacement still
calls the same cleanup method before presenting a restored workspace.

## Bounds and failure truth

- Window ID normalization, profile matching, selected-tab bounds, default-tab
  fallback, form database restoration, status text, and volatile-result cleanup
  moved unchanged.
- `restoreWindowIntentOnLaunch` became module-internal because initialization
  invokes it from the primary store file. No public API was added.
- `applySessionIntent` remains private to restoration ownership.
- No fixture symbol, I/O implementation, or generated FFI type entered the
  restoration extension.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures, including window/session intent
  restoration scenarios.
- Structural and runtime gates passed for multi-window ownership and query-tab
  isolation.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.

## Remaining work

Extract SQL files, tabs/navigation, transfer, and administration workflows.
Then enforce Presentation as a separate module with only Feature dependency.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
