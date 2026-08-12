# Native library-sheet extraction

Date: 2026-08-13

Checkpoint: P4a — extract history and saved-query presentation

## Decision

The history and saved-query SwiftUI sheets now live in `LibrarySheets.swift`.
They consume the history/saved-query workflow seam rather than remaining mixed
with the workbench shell, grids, AppKit adapters, fixtures, and connection UI.

This is the first production view extraction after store decomposition. It
creates a presentation-owned file boundary without adding another state owner.

## Bounds and failure truth

- Native lists, searchable state, toolbar pickers, save/delete dialogs,
  accessibility hints, loading/empty/error states, and sizing moved unchanged.
- Both sheets still receive the one `WorkbenchPresentationStore` through the
  SwiftUI environment. No backend call or persistence policy entered a view.
- No fixture symbol, generated FFI type, AppKit adapter, or Design Lab dependency
  entered the sheet file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- History structural/runtime gate passed.
- Saved-query structural/runtime gate passed.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 7,190 to 6,997 lines.

## Remaining work

Continue extracting workbench surfaces, sheets, and narrow AppKit adapters.
Then enforce the Presentation module and remove Release fixture/scripted-backend
membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
