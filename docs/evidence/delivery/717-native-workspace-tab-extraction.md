# Native workspace-tab extraction

Date: 2026-08-13

Checkpoint: P4t — extract query and object tab presentation

## Decision

Query/object tab selection, action menus, environment and safety labels, and
new-tab affordance now live in `WorkspaceTabs.swift`.

## Bounds and failure truth

- Selected styling, stable accessibility identifiers, rename/close/pin/
  refresh actions, running close guards, production/read-only facts, and the
  64-tab bound moved unchanged.
- Tab state, dirty/running truth, preview pinning, and close decisions remain in
  the single presentation store.
- No backend, fixture, generated FFI, AppKit, or Design Lab dependency entered
  the tab file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- `scripts/verify-native-query-tabs.sh` and
  `scripts/verify-native-object-tabs.sh` passed structural and runtime gates.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 3,753 to 3,599 lines.

## Remaining work

Extract object controls, review, PostgreSQL, inspector, and environment chrome.
Then enforce the Presentation module and remove Release fixture/scripted-
backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
