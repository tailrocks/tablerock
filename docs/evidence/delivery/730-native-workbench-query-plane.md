# Native Workbench production query plane

Date: 2026-08-13

Checkpoint: P9 — approved tabs, query/editor, results, and history composition

## Decision

Production query work now follows the operator-confirmed Native Workbench: a
compact query header with title, context, editor facts, Explain, and Run/Cancel;
an opaque AppKit SQL editor; and one resizable result plane with Results,
Messages, and Plan sections. Copy, export, paging, loaded-row filtering, saved
queries, SQL files, find/replace, and engine-specific query actions remain
available from compact native controls without coating editor or result content
in glass.

Explain now opens the in-workbench Plan section. The superseded modal plan
viewer and its duplicate presentation state were removed; its copy and
accessibility contracts moved into the Plan section.

## Ownership and invariants

- `QueryWorkbenchView` owns query composition only. It sends query, cancel,
  explain, library, file, and filter intents to the presentation store.
- `NativeQueryTab.selectedResultSection` owns the transient Results/Messages/Plan
  selection. Query execution chooses Results, failures choose Messages, and a
  successful explain chooses Plan.
- `SqlTextEditor` and `CatalogGrid` remain the narrow AppKit editing and dense
  table boundaries; no I/O entered rendering.
- `NativeWorkspaceTabReference` gives mixed query/object documents one stable
  visible order while query and object models retain their typed ownership.
  Reconciliation removes stale and duplicate references and appends live tabs
  that predate ordering state.
- The permanent status bar now projects the active workbench kind, rather than
  assuming that the existence of any object tab makes the object status active.
- Empty Run requests produce an explicit Messages-state error instead of a
  silent no-op. Rust remains authoritative for execution, cancellation,
  paging, history, export, and safety.

## Deterministic production evidence

`TABLEROCK_FIXTURE_NATIVE_WORKBENCH_QUERY=1` is an invented, stable,
development-support-only route. It uses the same Northstar Analytics projection
as the object-plane route, selects the query document, and supplies a populated
result without contacting a database. `TABLEROCK_FIXTURE_APPEARANCE=light`
forces Aqua for deterministic light evidence while scheme-only captures omit
fixture-label chrome. Release builds contain neither route.

Representative running-window capture:

- [`native-workbench__sql-results__light__postgresql__populated__typical__active.png`](../production/native-workbench/captures/native-workbench__sql-results__light__postgresql__populated__typical__active.png)

SHA-256:
`799448fcfc27c5bd620671b94fe257d7d9562baa7a7c9cf7dbc4d68788217f2f`

This is checkpoint evidence, not the final production state matrix.

## Verification

- `swift test --package-path native`: 58 XCTest cases passed, 3 expected
  live-server skips, 0 failures; 23 Swift Testing cases passed.
- `swift test --package-path native -c release`: 53 XCTest cases passed, 3
  expected live-server skips, 0 failures; 23 Swift Testing cases passed.
- Focused Xcode UI tests passed for the deterministic query workbench and the
  in-workbench Explain plan/copy workflow.
- `scripts/verify-native-query-tabs.sh` passed.
- `scripts/verify-native-history.sh` passed.
- `scripts/verify-native-saved-queries.sh` passed.
- `scripts/verify-native-sql-files.sh` passed.
- `scripts/verify-native-result-copy.sh` passed its shared Rust formatter,
  loaded export, and full streaming export runtime proof.
- `scripts/verify-native-accessibility.sh` passed.
- `scripts/verify-native-source-ownership.sh` passed, including the reviewed
  development-fixture inventory.
- `scripts/verify-native-performance.sh target/native-p9-performance` passed
  with 10,000 resident rows, 1.654282-second automated scroll, 173,024 KiB
  maximum RSS, and a 22,368,256-byte Time Profiler trace.
- `scripts/build-native-app.sh` produced and signed `native/dist/TableRock.app`.
- `swift-format lint` for changed production presentation/test sources and
  `git diff --check` passed.

## Clean-room provenance

Apple system toolbar, menu, split-view, editor, result table, status, and sheet
conventions plus the operator-confirmed TableRock Native Workbench runtime and
capture govern this implementation. TablePro's public user-visible interface
informed broad workbench organization only. No TablePro source, tests,
comments, bundle internals, branding, assets, copy, or proprietary fixtures
were used.
