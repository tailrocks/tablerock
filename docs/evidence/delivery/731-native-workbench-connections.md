# Native Workbench production connections

Date: 2026-08-13

Checkpoint: P8 — approved connection browser and setup plane

## Decision

Production connection management now follows the operator-confirmed Native
Workbench. Before a session opens, the stable leading plane contains native
Connections and Connection Setup routes, recent saved profiles, Settings, and
the new-connection action. The opaque detail plane contains either a searchable
adaptive connection browser or a full connection-setup workspace with a live
summary and persistent Test Connection and Save & Connect actions.

New, imported, and edited connections remain native sheets. Temporary
connections also remain a native sheet. The presentation migration did not
move connection, secret, safety, persistence, or database semantics out of
Rust and the existing bridge-facing workflows.

## Ownership and invariants

- `ConnectionsNavigatorPane` owns disconnected navigation composition. One
  typed list-selection model covers route and recent-profile rows, preventing
  mixed selection identities from suppressing native list content.
- `ConnectionWorkspaceSurface` and `ProfileEditorPresentation` make browser,
  workspace, and sheet routing explicit in `WorkbenchPresentationStore`.
- `ConnectionsBrowserView` owns search, adaptive cards, empty/loading/error
  states, and entry actions. Each connection card is a native button with a
  sibling native actions menu; controls are never nested.
- `ProfileEditorSheet` owns the shared form. Workspace and sheet layouts share
  draft validation and store intents while keeping their chrome distinct.
- Setup and empty-state identifiers live on leaf headings. Container-level
  identifiers were removed because SwiftUI propagated them to descendants and
  erased actionable accessibility identities.
- The disconnected root observes profile readiness so its customizable unified
  toolbar installs even when initialization keeps the Connections route
  selected.
- Dense query/object tabs again project production/environment and read-only
  facts. Their accessible label includes those facts without relying on color.
- Forms and profile cards remain opaque CONTENT. Liquid Glass stays with the
  system-owned sidebar, toolbar, search field, and standard controls.

## Deterministic production evidence

Two invented development-support-only routes exercise the approved production
composition without network access:

- `TABLEROCK_FIXTURE_NATIVE_WORKBENCH_CONNECTIONS=1`
- `TABLEROCK_FIXTURE_NATIVE_WORKBENCH_SETUP=1`

Both use invented Northstar Analytics, Atlas Events, and Arbor Cache profiles.
Release builds contain neither route.

Representative running-window captures:

- [`native-workbench__connections__light__postgresql__populated__typical__active.png`](../production/native-workbench/captures/native-workbench__connections__light__postgresql__populated__typical__active.png)
- [`native-workbench__setup__light__postgresql__populated__typical__active.png`](../production/native-workbench/captures/native-workbench__setup__light__postgresql__populated__typical__active.png)

SHA-256:

- Connections: `329e077596bbb5d93464b189b5f9e84b238d8a177ac48f7b6251bffdac9ae5a2`
- Setup: `4eaeb9c8f9a53c50c665944f7fd60a059d0308e036ce68c063bf6b0c4c870f0b`

These are checkpoint captures, not the final production state matrix.

## Verification

- `swift test --package-path native`: 58 XCTest cases passed, 3 expected
  live-server skips, 0 failures; 23 Swift Testing cases passed.
- Focused Xcode UI tests passed for the deterministic connection browser,
  full setup workspace, profile create/save flow, and temporary connection
  sheet.
- `scripts/verify-native-profile-editor.sh` passed its structural and runtime
  proof.
- `scripts/verify-native-profile-groups.sh` passed after restoring
  environment/safety facts to the approved dense tabs.
- `scripts/verify-native-accessibility.sh` passed.
- `scripts/verify-native-source-ownership.sh` passed, including the reviewed
  development-fixture inventory.
- `scripts/build-native-app.sh` produced and signed `native/dist/TableRock.app`.
- `swift-format lint` for changed Swift sources and `git diff --check` passed.

## Clean-room provenance

Apple system sidebar, list, toolbar, search, form, sheet, button, menu, and
split-view conventions plus the operator-confirmed TableRock Native Workbench
runtime and captures govern this implementation. TablePro's public
user-visible interface informed broad connection-workbench organization only.
No TablePro source, tests, comments, bundle internals, branding, assets, copy,
or proprietary fixtures were used.
