# Native workbench navigation extraction

Date: 2026-08-13

Checkpoint: P3i — extract tab and object-navigation workflows

## Decision

Query-tab lifecycle, object-tab lifecycle, catalog-object opening, browse
sorting/filtering, filter presets, Redis key paging, and relation-structure
loading now live in `WorkbenchPresentationStore+Navigation.swift`.

The store remains the sole presentation-state owner. The extraction creates a
coherent navigation seam before the confirmed Native Workbench shell replaces
the legacy view composition.

## Bounds and failure truth

- Tab limits, unsaved/running close guards, preview pinning, persistence intent,
  browse execution, paging, filters, and error strings moved unchanged.
- `loadObjectTab` and `loadObjectFilterPresets` became module-internal because
  fixture initialization in the primary declaration still invokes them. This
  is temporary Release fixture debt already tracked by the source-ownership
  gate; no public API was added.
- Database behavior remains behind `WorkbenchBackend`; Rust retains catalog,
  query, paging, and structure authority.
- No fixture symbol, generated FFI type, or Design Lab dependency entered the
  navigation extension.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- Query-tab and object-tab structural/runtime gates passed.
- PostgreSQL and ClickHouse structure structural/runtime gates passed.
- Redis key-view structural/runtime gate passed.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.

## Remaining work

Extract transfer and administration workflows. Then enforce the Presentation
module and remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
