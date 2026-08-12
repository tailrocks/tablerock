# Native find/replace-engine extraction

Date: 2026-08-13

Checkpoint: P4j — remove editor transformation logic from the view file

## Decision

The bounded find/replace engine, typed errors, and replacement outcome now live
in `NativeFindReplaceEngine.swift` rather than the legacy view file.

The engine is Foundation-only. SwiftUI dialog orchestration remains in the
editor workflow extension, and AppKit text input remains behind the editor
adapter.

## Bounds and failure truth

- Literal, case-sensitive, whole-word, and regular-expression matching,
  selection scope, forward/backward traversal, composed-character progress,
  replacement templates, scope adjustment, and the 10,000-match bound moved
  unchanged.
- No SwiftUI, AppKit, backend, fixture, generated FFI, or Design Lab dependency
  entered the engine file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- Focused find/replace behavior passed modes, scopes, and zero-width regular
  expressions.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 5,612 to 5,460 lines.

## Remaining work

Extract result surfaces, sheets, and narrow AppKit adapters. Then enforce the
Presentation module and remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
