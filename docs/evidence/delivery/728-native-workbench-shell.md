# Native Workbench production shell

Date: 2026-08-13

Checkpoint: P7 — approved shell, windows, commands, and toolbar

## Decision

The operator-confirmed Native Workbench now governs the production macOS
shell. A connected window is a balanced `NavigationSplitView` with one
persistent catalog sidebar and a full-bleed workbench plane: compact connection
context, opaque document tabs, dominant content, permanent status, and an
optional full-height trailing value inspector. The system window uses a hidden
title bar and unified toolbar at a production minimum size of 1280 × 680.

The prior hierarchy put saved connection profiles above the connected catalog,
duplicated surface commands in a crowded system toolbar, padded the workbench
like a settings form, and nested the inspector inside the grid. The enabling
condition was presentation ownership split across independent sidebar,
toolbar, tab, content, and grid-local inspector layouts. The shell now owns
their composition; existing stores and command intents remain authoritative.

## Behavior preserved

- connection switching, disconnect, catalog refresh, quick switcher, new
  query, history, inspector, and change-review actions still dispatch existing
  presentation intents;
- run/cancel remain discoverable Query menu commands with existing shortcuts;
- query/object selection, rename, close, pin, refresh, dirty state, running
  state, and the 64-tab bound remain intact;
- connected catalog filtering is real and preserves both ancestors and matching
  descendants;
- disconnected windows retain the saved-profile browser and connection flow;
- value inspection still projects typed cells and now closes from the
  full-height trailing inspector.

## Structural guard

The accessibility verifier now checks the command-menu labels used by the
navigation-only toolbar and requires the Native Workbench new-query and
inspector toolbar items. It no longer requires the rejected toolbar's fixed
glass-cluster spacers. The Feature layering test records that document tabs are
opaque content chrome, never glass pills.

## Evidence

- `swift test --package-path native -c release`: 53 XCTest cases passed, 3
  expected live-server skips, 0 failures; 23 Swift Testing cases passed.
- `scripts/verify-native-source-ownership.sh` passed.
- `scripts/verify-native-multi-window.sh` passed.
- `scripts/verify-native-object-tabs.sh` passed.
- `scripts/verify-native-query-tabs.sh` passed.
- `scripts/verify-native-accessibility.sh` passed, including the runtime
  accessibility/catalog-state proof.
- `git diff --check` passed.

## Clean-room provenance

Apple system window, toolbar, split-view, sidebar, menu, and inspector
conventions plus the operator-confirmed TableRock Native Workbench runtime and
capture govern this implementation. TablePro's public user-visible interface
informed broad workbench organization only. No TablePro source, tests,
comments, bundle internals, branding, assets, copy, or proprietary fixtures
were used.
