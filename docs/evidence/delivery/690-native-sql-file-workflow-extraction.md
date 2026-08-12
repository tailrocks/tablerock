# Native SQL file workflow extraction

Date: 2026-08-13

Checkpoint: P3g — extract SQL file workflows

## Decision

Unsaved-editor detection, open/save panel orchestration, security-scoped file
access, external-change confirmation, reload, and SQL-file status now live in
`WorkbenchPresentationStore+Files.swift`.

The extension uses application file-panel ports and the bridge-neutral backend.
It does not perform raw file I/O, persist editor contents itself, or weaken the
existing external-modification check.

## Bounds and failure truth

- File extensions, panel labels, security-scope lifetime, expected metadata,
  overwrite authority, status messages, and typed Bridge error handling moved
  unchanged.
- SQL baseline and error projections became module-internal for same-type
  cross-file access. No public API was added.
- I/O remains outside SwiftUI update/render paths.
- No fixture symbol, generated FFI type, or database policy entered the file
  extension.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures, including file-panel
  confinement and SQL-file behavior.
- `scripts/verify-native-sql-files.sh` passed structural and runtime proof.
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
