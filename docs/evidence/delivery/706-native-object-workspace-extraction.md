# Native object-workspace extraction

Date: 2026-08-13

Checkpoint: P4i — extract object workbench surface

## Decision

The production object workspace now lives in `ObjectWorkbenchView.swift`.
Object identity, data/structure switching, pin/refresh/import/close controls,
loading/error/empty states, sort/filter composition, Redis key paging, result
paging, and structure presentation form one object-owned surface.

The existing sort, filter, Redis, and structure helper views remain private in
their current file. Narrow opaque-view factories expose only their composition
boundary to the extracted workspace.

## Bounds and failure truth

- Object-tab lifecycle, native section picker, action enablement, paging,
  structure load, result integration, and state messages moved unchanged.
- Liquid Glass remains limited to functional object controls. Data, structure,
  Redis values, and result content remain opaque.
- Object browsing, filtering, import, paging, and structure authority remain
  behind workflows/backend.
- No fixture symbol, generated FFI type, backend import, or Design Lab
  dependency entered the object workspace file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- Object-tab structural/runtime gate passed.
- PostgreSQL structure structural/runtime gate passed.
- Redis key-view structural/runtime gate passed.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 5,693 to 5,612 lines.

## Remaining work

Extract result surfaces, sheets, and narrow AppKit adapters. Then enforce the
Presentation module and remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
