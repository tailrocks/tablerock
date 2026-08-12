# Native Workbench refined confirmation gate

Status: **PHASE 11 — OPERATOR CONFIRMED**

Implementation revision: `73dfd349084a9d13a0f0437656542a318f69e4df`

Captured: 2026-08-12 on macOS 26.5.2, Xcode 26.6, macOS SDK 26.5

This is the completed second native-design gate. Native Workbench completed its
presentation-only refinement in `TableRockDesignLab`. Production UI sources
remained unchanged through confirmation.

## Preview the exact build

```sh
./scripts/verify-native-design-lab.sh
open -n "$PWD/target/design-lab-derived-data/Build/Products/Debug/TableRockDesignLab.app" \
  --args --concept native-workbench --surface data-grid \
  --appearance light --accessibility system --engine postgresql \
  --fixture populated --window-size typical
```

The captured implementation is commit
`73dfd349084a9d13a0f0437656542a318f69e4df`. The capture
[manifest](captures/MANIFEST.tsv), complete
[inventory and launch arguments](captures/CAPTURES.tsv), and
[SHA-256 checksums](captures/SHA256SUMS) bind 26 real-window captures to that
revision.

Sketch approved frames: **none**. The operator rejected and purged
`design/TableRock-Native-Concepts.sketch`; it is not a reference or authority.
The running Design Lab and the captures below are the design authority for this
gate.

## Representative runtime captures

| Evidence | Capture |
|---|---|
| Main workbench | [Data grid](captures/native-workbench__data-grid__light__system__postgresql__populated__typical__active.png) |
| Dark appearance | [Dark data grid](captures/native-workbench__data-grid__dark__system__postgresql__populated__typical__active.png) |
| Connection browser | [Connections](captures/native-workbench__connections__light__system__postgresql__populated__typical__active.png) |
| Native connection sheet | [New Connection](captures/native-workbench__connections__light__system__postgresql__populated__typical__active__connection-sheet.png) |
| Connection setup | [Setup](captures/native-workbench__setup__light__system__postgresql__populated__typical__active.png) |
| Object structure | [Structure](captures/native-workbench__data-grid__light__system__postgresql__populated__typical__active__structure.png) |
| Query and results | [SQL + Results](captures/native-workbench__sql-results__light__system__postgresql__populated__typical__active.png) |
| Query failure | [Query error](captures/native-workbench__sql-results__light__system__postgresql__populated__typical__active__query-error.png) |
| Query history | [History](captures/native-workbench__sql-results__light__system__postgresql__populated__typical__active__query-history.png) |
| Safe row edit | [Edit sheet](captures/native-workbench__data-grid__light__system__postgresql__populated__typical__active__safe-edit.png) |
| Safe write review | [Safe review](captures/native-workbench__data-grid__light__system__postgresql__populated__typical__active__safe-review.png) |
| Destructive write review | [Destructive review](captures/native-workbench__data-grid__light__system__postgresql__destructive-review__typical__active.png) |
| Pending changes | [Change Review](captures/native-workbench__change-review__light__system__postgresql__pending-change__typical__active.png) |
| Minimum size | [1280×760](captures/native-workbench__data-grid__light__system__postgresql__populated__minimum__active.png) |
| Expanded size | [1720×1040](captures/native-workbench__data-grid__light__system__postgresql__populated__expanded__active.png) |

## Interaction walkthrough

All behavior is deterministic presentation state. It performs no network,
database, persistence, or production action.

| Required flow | Refined behavior and evidence |
|---|---|
| Launch | Opens Native Workbench through deterministic arguments; window model test verifies restored size. |
| Create/open connection | Connections browser, native New Connection sheet, and full setup surface are operable. |
| Select database context | Toolbar connection menu changes session-owned engine and catalog context. |
| Browse schema | Persistent leading catalog exposes tables, views, functions, and types. |
| Open table | Catalog selection activates an object document and its data surface. |
| Inspect and sort rows | Native `NSTableView` selection updates the inspector; sort menu changes rows and visible sort state. |
| Select/inspect value | Selected cell and row details appear in the trailing inspector. |
| Open structure | Data/Structure control switches to columns, indexes, constraints, and relations. |
| Create/execute query | `Command-T` creates a query; editable SQL runs with `Command-Return` or Run. |
| Inspect results | Query/result split preserves editor, status, result modes, and dense native table. |
| Navigate history | Toolbar/menu history action opens local deterministic query history. |
| Perform safe edit | Edit Selected opens a row sheet; changes stage without execution. |
| Review pending changes | Safe and destructive sheets show exact operations before Apply becomes available. |
| Handle error | Empty SQL produces an inline error and Not Run status without losing prior results. |
| Switch engines | PostgreSQL, ClickHouse, and Redis update engine/catalog context without backend access. |
| Keyboard commands | Native menus expose search, new connection, new query, run, history, and inspector commands. |
| Resize/restore window | 1280×760, 1440×900, and 1720×1040 layouts pass; last size restores from local window defaults. |

The five UI tests operate the core flow end to end: select/sort/open an object,
open Structure, stage/review/apply a safe edit, create/run a query, open query
history, switch engines, exercise commands and sheets, audit accessibility, and
verify resize restoration. Ten model tests cover session transitions and
deterministic launch state.

## Required state evidence

| State family | Evidence |
|---|---|
| Empty/loading/error | [Empty](captures/native-workbench__data-grid__light__system__postgresql__empty__typical__active.png), [loading](captures/native-workbench__data-grid__light__system__postgresql__loading__typical__active.png), [connection error](captures/native-workbench__data-grid__light__system__postgresql__connection-error__typical__active.png) |
| Selection | [Selected cell](captures/native-workbench__data-grid__light__system__postgresql__selected-cell__typical__active.png) |
| Appearance | [Light](captures/native-workbench__data-grid__light__system__postgresql__populated__typical__active.png), [dark](captures/native-workbench__data-grid__dark__system__postgresql__populated__typical__active.png), [inactive](captures/native-workbench__data-grid__light__system__postgresql__populated__typical__inactive.png) |
| Accessibility | [Reduce Transparency](captures/native-workbench__data-grid__light__reduce-transparency__postgresql__populated__typical__active.png), [Increase Contrast](captures/native-workbench__data-grid__light__increase-contrast__postgresql__populated__typical__active.png), [Reduce Motion](captures/native-workbench__data-grid__light__reduce-motion__postgresql__populated__typical__active.png) |
| Engines | [PostgreSQL](captures/native-workbench__data-grid__light__system__postgresql__populated__typical__active.png), [ClickHouse](captures/native-workbench__data-grid__light__system__clickhouse__populated__typical__active.png), [Redis](captures/native-workbench__data-grid__light__system__redis__populated__typical__active.png) |

## Component ownership

| Owner | Responsibility |
|---|---|
| SwiftUI/AppKit native | Window, toolbar, menus, commands, search, buttons, pickers, segmented controls, sheets, focus, split navigation, accessibility semantics. |
| `NSTableView` adapter | Dense data/results grid, row selection, column resize/reorder, scrolling, context and sort menus. |
| Native-composed TableRock views | Catalog, document tabs, object/query headers, filter/status rails, structure view, value inspector, connection forms, review content. |
| Design Lab session | Local catalog/object/query/sort/edit/history/engine/presentation state and deterministic routes. |
| Production application | No ownership change; no Design Lab state, fixture, or UI code enters production. |

The trailing utility pane is a stable native-composed region. It intentionally
does not use SwiftUI `.inspector`: that presentation produced AppKit safe-area
split failures under the required dense table layout. This keeps ownership
clear while retaining native controls and semantics.

## Material and Liquid Glass audit

| Material class | Regions and rule |
|---|---|
| System functional chrome | Window/titlebar, toolbar groups, sidebar chrome, menus, popovers, and sheets own platform material. |
| Opaque content | Tables, SQL editor, results, structure, inspector values, connection values, and write-review details stay opaque. |
| Semantic emphasis | System selection, tint, status badges, filter tokens, and destructive/safe emphasis communicate state without decorative glass. |
| Accessibility fallback | Reduce Transparency removes translucency; Increase Contrast strengthens boundaries; Reduce Motion suppresses decorative transitions. |

Audit result: **pass, no hard failure**. Glass is confined to system-owned
functional chrome. No data, SQL, credential, result, inspector, or change-review
content sits on custom glass. Layering, contrast, inactive state, and material
fallbacks remain coherent across the capture matrix.

## Visual and native-design review

Runtime review used the 26-capture matrix at original resolution, including
full-size inspection of minimum sizing, Structure, connection and edit sheets,
query failure, and destructive review.

- Information hierarchy remains readable at 1280×760 without clipped controls
  or hidden safety state.
- Dense native tables preserve scanability, selection, sorting, and column
  behavior in light and dark appearances.
- Sheets separate creation/edit/review intent and keep irreversible action
  language explicit.
- The catalog, document tabs, work surface, and utility pane preserve database
  orientation through browse, query, and edit flows.
- Empty, loading, connection-error, and query-error states preserve recovery
  context instead of replacing the whole workbench.
- No hard failure remains in hierarchy, contrast, resizing, interaction,
  accessibility semantics, or material ownership.

## Accessibility verification

The verifier passed ten model tests and five XCUITests. The runtime audit checks
named navigation, workbench, toolbar, content, table, inspector, status, sheet,
and menu regions/actions. The end-to-end test operates the real native sort menu
and core flow. Increase Contrast, Reduce Transparency, Reduce Motion, inactive
window, minimum window, and dark appearance all have deterministic capture
evidence.

## Differences from the selected first-gate concept

- Catalog selection, engine context, sort state, object mode, query history,
  and window restoration are now session-owned rather than illustrative only.
- Data and Structure are distinct operable modes.
- Safe row editing stages exact changes, followed by safe or typed destructive
  review before any local simulated apply.
- Query creation, editing, success, failure, results, and history form one
  coherent local flow.
- The capture surface now has deterministic presentation routes for every
  confirmation-critical sheet and error state.
- The utility pane uses a stable native-composed layout after removing an
  AppKit safe-area failure caused by the earlier inspector composition.

## Verification record

```sh
./scripts/verify-native-design-lab.sh
./scripts/capture-native-design-lab.sh \
  docs/evidence/design-lab/native-workbench-refined/captures refined
shasum -a 256 -c docs/evidence/design-lab/native-workbench-refined/captures/SHA256SUMS
git diff origin/main -- native/Sources/TableRockApp \
  native/Sources/TableRockFeature native/Sources/TableRockBridge
```

Results:

- independent Xcode dependency graph: empty;
- build: passed;
- model tests: 10 passed;
- XCUITests: 5 passed;
- runtime accessibility audit: passed;
- capture checksums: 26 passed;
- production-source diff from `origin/main`: empty.

## Unresolved limits

- This is static, invented, presentation-only state. Database, persistence,
  credential, SQL execution, safety enforcement, and UniFFI integration remain
  production work governed by later roadmap gates.
- Accessibility variants are deterministic in-process previews. Final
  production validation must also run with real macOS accessibility settings,
  keyboard-only operation, VoiceOver, and representative user data.
- Long and localized production content still needs later boundary testing.
- This gate approves a design direction only if the operator explicitly
  confirms it; it does not authorize hidden production migration.

## Clean-room provenance

TablePro informed broad, user-visible workbench organization: persistent
database context, dense native results, document/query tabs, inspectors,
connection sheets, and keyboard-first flow. Apple platform behavior and the
installed stable SDK govern implementation. All names, fixtures, geometry,
copy, and code are original TableRock work. No TablePro source, tests, comments,
bundle internals, branding, or proprietary assets were inspected or used. The
rejected Sketch artifact had no influence.

## Operator confirmation

The operator confirmed this exact refined concept on 2026-08-12 by stating:

> This native concept is the production design TableRock should implement.

Production architecture planning and migration are therefore authorized. The
Design Lab remains reference evidence and must not become a production
dependency.
