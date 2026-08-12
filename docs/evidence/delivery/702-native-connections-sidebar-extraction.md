# Native connections-sidebar extraction

Date: 2026-08-13

Checkpoint: P4e — extract connection navigation sidebar

## Decision

The connection profile list now lives in `ConnectionsProfileList.swift`.
Grouped navigation, search, active-profile selection, profile and group menus,
native context menus, functional Liquid Glass bottom controls, and
loading/error/empty states form one sidebar-owned surface.

## Bounds and failure truth

- Group collapse/order, delayed search, connect/health/reconnect/disconnect,
  edit/duplicate/test/favorite/move/remove actions, sample/new/group/URL controls,
  accessibility identifiers, and empty-state actions moved unchanged.
- The sidebar consumes the environment-owned presentation store. Backend calls
  remain in workflow methods; no persistence, session, or credential authority
  entered the view.
- Liquid Glass remains limited to the functional bottom control group. List
  content remains on the native sidebar content surface.
- No fixture symbol, generated FFI type, backend import, AppKit adapter, or
  Design Lab dependency entered the sidebar file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- Profile-group structural/runtime gate passed.
- Profile-editor structural/runtime gate passed through sidebar launch actions.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 6,365 to 6,132 lines.

## Remaining work

Extract the live catalog pane, workbench surfaces, and narrow AppKit adapters.
Then enforce the Presentation module and remove Release fixture/scripted-backend
membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
