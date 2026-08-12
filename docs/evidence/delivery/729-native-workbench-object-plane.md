# Native Workbench production object plane

Date: 2026-08-13

Checkpoint: P9 — approved object/data/structure work plane

## Decision

Production object browsing now follows the operator-confirmed Native Workbench:
a compact object header, one filter/sort rail, a dominant opaque data grid, an
optional full-height value inspector, and a permanent Data/Structure status
rail. Raw WHERE, saved presets, and loaded-row filtering remain available from
compact popovers instead of occupying the data plane.

Structure mutation and table-operation entry points now belong to the object
header's action menu. This removes a text-wide action control from horizontally
constrained structure content and keeps mutation discovery in one stable
location.

## Ownership and behavior

- `ObjectWorkbenchView` owns object composition and its compact action header.
- `ObjectBrowseRail` owns visible filter tokens, filter popovers, sort menus,
  loaded-row filtering, and clear actions.
- `CatalogGrid` remains the narrow AppKit-backed dense-grid boundary. It now
  restores model selection without feeding programmatic selection back into
  SwiftUI observation. This removes the constraint-update loop exposed by the
  deterministic selected-cell route.
- `ResultSurfaces` owns reusable copy/export controls and the full-height value
  inspector; object browsing suppresses duplicate grid-local utility chrome.
- `WorkbenchChrome` owns the Data/Structure selector, row/filter/column/page
  facts, and pending-change status.
- Existing Rust/UniFFI intents continue owning browse, reload, filtering,
  mutation review, safety, and database semantics.

## Deterministic production evidence

`TABLEROCK_FIXTURE_NATIVE_WORKBENCH=1` is an invented, stable,
development-support-only route. It projects a PostgreSQL workbench with catalog
objects, document tabs, two active filters, one descending sort, twelve rows,
a selected value, and an open inspector. Release builds contain neither its
configuration nor its fixture construction.

Representative running-window capture:

- [`native-workbench__object-data__light__postgresql__populated__typical__active.png`](../production/native-workbench/captures/native-workbench__object-data__light__postgresql__populated__typical__active.png)

This is checkpoint evidence, not the final production state matrix.

## Verification

- `swift test --package-path native`: 56 XCTest cases passed, 3 expected
  live-server skips, 0 failures; 23 Swift Testing cases passed.
- `swift test --package-path native -c release`: 53 XCTest cases passed, 3
  expected live-server skips, 0 failures; 23 Swift Testing cases passed.
- Focused Xcode UI tests passed independently for the deterministic production
  workbench, sort/filter/preset workflow, frozen structure review, and exact
  table-operation confirmation.
- `scripts/verify-native-object-tabs.sh` passed.
- `scripts/verify-native-accessibility.sh` passed.
- `scripts/verify-native-source-ownership.sh` passed, including reviewed
  development-fixture inventory.
- `scripts/build-native-app.sh` produced and signed `native/dist/TableRock.app`.
- `git diff --check` passed.

## Clean-room provenance

Apple system table, menu, popover, split-view, inspector, selection, and status
conventions plus the operator-confirmed TableRock Native Workbench runtime and
capture govern this implementation. TablePro's public user-visible interface
informed broad workbench organization only. No TablePro source, tests,
comments, bundle internals, branding, assets, copy, or proprietary fixtures
were used.
