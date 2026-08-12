# Native workbench-root extraction

Date: 2026-08-13

Checkpoint: P4n — extract root navigation and presentation hosting

## Decision

The production `NavigationSplitView`, workbench/welcome selection, sheet and
dialog presentation, initialization task, focused command projection, and
toolbar attachment now live in `WorkbenchRootView.swift`.

Sheet entry types became module-internal so root composition can reference
them across focused source files. Their state and behavior did not change.

## Bounds and failure truth

- Connection/catalog split behavior, every sheet and dialog binding, dismissal
  cleanup, destructive role, initialization timing, and focused command
  capabilities moved unchanged.
- Root composition still derives only from the single presentation store.
- No backend, fixture, generated FFI, or Design Lab dependency entered the
  root view.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- `scripts/verify-native-multi-window.sh` passed its structural and runtime
  gate.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 4,744 to 4,496 lines.

## Remaining work

Extract editor, transfer, engine, review, inspector, tab, toolbar, and settings
surfaces. Then enforce the Presentation module and remove Release
fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
