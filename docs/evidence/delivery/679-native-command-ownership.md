# Native command ownership

Date: 2026-08-12

Checkpoint: P2d — extract commands and focused actions

## Decision

`WorkbenchCommands`, its focused-value key, and the derived
`WorkbenchActions` capability projection now live in
`native/Sources/TableRockApp/WorkbenchCommands.swift`.

Commands derive enablement from the window-owned `BridgeModel` and forward
typed operator intents to that model. They own no bridge, database, safety, or
independent mutable state. Moving the focused-value projection with its command
consumer prevents command enablement from drifting away from menu behavior.

## Bounds and failure truth

- Menu labels, keyboard shortcuts, enablement conditions, and dispatched model
  intents did not change.
- `WorkbenchActions` and the focused-value property became module-internal only
  because `ContentView` publishes them from a separate file. No public API was
  added.
- Toolbar presentation remains in the monolith and will move with the
  presentation boundary, not into command ownership.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `scripts/verify-native-accessibility.sh` passed structural and runtime proof,
  including command/toolbar labels and operative workbench state.
- `TableRockAppTests`: 21 passed, zero failures.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target with `WorkbenchCommands.swift`.
- `git diff --check` passed.

## Remaining work

Extract presentation drafts and backend implementations, then establish the
compiler-enforced Presentation/Feature/Bridge dependency graph.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
