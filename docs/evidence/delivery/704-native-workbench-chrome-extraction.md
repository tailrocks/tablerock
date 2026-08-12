# Native workbench-chrome extraction

Date: 2026-08-13

Checkpoint: P4g — extract workbench shell and chrome

## Decision

The connected workbench shell, disconnected welcome/direct-connect surface,
context strip, and permanent status bar now live in `WorkbenchChrome.swift`.

This boundary owns the confirmed Native Workbench composition: native content
switching between query and object workspaces, dense context facts, functional
controls, and persistent operation/safety facts.

## Bounds and failure truth

- Shell ordering, native direct-connect form, context/session/engine facts,
  catalog refresh, tab/content selection, operation summary, Change Ledger and
  production warnings, and accessibility semantics moved unchanged.
- `EnvironmentSafetyBadge` became module-internal so the extracted context strip
  can compose it. No public API was added.
- Liquid Glass remains on functional controls only. Workbench content and status
  facts remain opaque native content surfaces.
- No fixture symbol, generated FFI type, backend import, AppKit adapter, or
  Design Lab dependency entered the chrome file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- Query-tab structural/runtime gate passed.
- Multi-window structural/runtime gate passed.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 6,065 to 5,830 lines.

## Remaining work

Extract query/object workbench surfaces, sheets, and narrow AppKit adapters.
Then enforce the Presentation module and remove Release fixture/scripted-backend
membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
