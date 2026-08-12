# Native SQL-editor adapter extraction

Date: 2026-08-13

Checkpoint: P4k — isolate AppKit SQL text input from the legacy view file

## Decision

The SQL editor `NSViewRepresentable`, its `NSTextView` coordinator, and the
line-number ruler now live in `SqlTextEditor.swift`.

This creates one narrow AppKit ownership boundary for text input while keeping
SwiftUI query composition in `QueryWorkbenchView.swift` and editor workflow
state in the presentation store.

## Bounds and failure truth

- Text, selection, marked-text preservation, undo, find-bar behavior,
  accessibility identifiers, running-state color, scroll behavior, and line
  numbering moved unchanged.
- No backend, fixture, generated FFI, or Design Lab dependency entered the
  adapter.
- The adapter depends only on AppKit, SwiftUI, and the stable
  `TableRockFeature.SqlEditorMetrics` helper.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- `scripts/verify-native-query-tabs.sh` passed its structural and runtime gate.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 5,460 to 5,267 lines.

## Remaining work

Extract the catalog outline and grid adapters, then enforce the Presentation
module and remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
