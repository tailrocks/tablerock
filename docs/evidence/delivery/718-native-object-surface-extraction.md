# Native object-surface extraction

Date: 2026-08-13

Checkpoint: P4u — extract object browse and structure presentation

## Decision

Object sort/filter/raw-WHERE/preset controls, Redis key values, and relational
structure presentation now live in `ObjectBrowseSurfaces.swift`.

## Bounds and failure truth

- Sort direction/order, typed filter operators, 16-sort and 32-filter bounds,
  65,536-byte raw-WHERE bound, saved presets, Redis paging surface, structure
  loading/error/empty states, database facts, DDL copy, and reviewed operation
  entry points moved unchanged.
- SQL generation, typed browse semantics, persisted presets, structure truth,
  and reviewed mutations remain below presentation.
- No backend, fixture, generated FFI, AppKit, or Design Lab dependency entered
  the object-surface file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- Object-tab, PostgreSQL structure, ClickHouse structure, and Redis key-view
  structural/runtime gates passed.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 3,599 to 3,214 lines.

## Remaining work

Extract review, PostgreSQL, inspector, and environment chrome. Then enforce
the Presentation module and remove Release fixture/scripted-backend
membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
