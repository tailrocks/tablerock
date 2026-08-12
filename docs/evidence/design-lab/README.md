# Native Design Lab operator gate

Status: **SELECTED — refined confirmation gate open**

Captured revision: `f0820edc01be011ca241654d783ad6ee46ee35c6`  
Captured: 2026-08-12 on macOS 26.5.2, Xcode 26.6, macOS SDK 26.5

This records the completed first design gate. The operator selected Native
Workbench on 2026-08-12 after launching the current Design Lab preview. The five
concepts remain runnable, interaction-capable, dependency-isolated, and captured
from the real Design Lab window. Production `TableRockApp` UI remains unchanged.

Phase 10 refinement is complete. The second operator gate, exact refined build,
and 26-capture evidence matrix are available in the
[Native Workbench refined confirmation gate](native-workbench-refined/README.md).

## Run

```sh
./scripts/verify-native-design-lab.sh
open target/design-lab-derived-data/Build/Products/Debug/TableRockDesignLab.app
```

Deterministic example:

```sh
open -n "$PWD/target/design-lab-derived-data/Build/Products/Debug/TableRockDesignLab.app" \
  --args --concept native-workbench --surface data-grid --appearance dark \
  --accessibility system --engine clickhouse --fixture pending-change \
  --window-size typical --capture
```

The in-app **Design Lab** menu changes concept, surface, fixture, appearance,
accessibility preview, engine, and window size. `Command-T` opens a query,
`Shift-Command-N` opens connection setup, and `Command-Return` runs the static
query route.

## Concepts and representative runtime captures

| Concept | Structure | Representative capture |
|---|---|---|
| Native Workbench | Catalog + document workspace + optional inspector | [Data Grid](captures/native-workbench__data-grid__light__system__postgresql__populated__typical__active.png) |
| Query Studio | Mode rail + editor-led vertical workspace | [Data Grid](captures/query-studio__data-grid__light__system__postgresql__populated__typical__active.png) |
| Column Observatory | Persistent source, object, content, and inspector regions | [Data Grid](captures/column-observatory__data-grid__light__system__postgresql__populated__typical__active.png) |
| Grid Canvas | Content-first canvas + detached command/navigation palettes | [Data Grid](captures/grid-canvas__data-grid__light__system__postgresql__populated__typical__active.png) |
| Change Desk | Compact navigation + persistent staged-change ledger | [Data Grid](captures/change-desk__data-grid__light__system__postgresql__populated__typical__active.png) |

Every concept also has real-window captures for Connections, Connection Setup,
SQL + Results, Change Review, dark work surfaces, and inactive-window state.
The full 55-image inventory and exact launch arguments are in
[`CAPTURES.tsv`](captures/CAPTURES.tsv); hashes are in
[`SHA256SUMS`](captures/SHA256SUMS).

Useful state evidence:

- [Minimum 1280×760](captures/native-workbench__data-grid__light__system__postgresql__populated__minimum__active.png)
- [Expanded 1720×1040](captures/native-workbench__data-grid__light__system__postgresql__populated__expanded__active.png)
- [Dark SQL + Results](captures/native-workbench__sql-results__dark__system__postgresql__populated__typical__active.png)
- [Reduce Transparency](captures/native-workbench__data-grid__light__reduce-transparency__postgresql__populated__typical__active.png)
- [Increase Contrast](captures/native-workbench__data-grid__light__increase-contrast__postgresql__populated__typical__active.png)
- [Connection error](captures/native-workbench__data-grid__light__system__postgresql__connection-error__typical__active.png)
- [Long identifiers](captures/native-workbench__data-grid__light__system__postgresql__long-identifiers__typical__active.png)
- [Pending change](captures/native-workbench__data-grid__light__system__postgresql__pending-change__typical__active.png)
- [Destructive review](captures/native-workbench__data-grid__light__system__postgresql__destructive-review__typical__active.png)
- [ClickHouse](captures/native-workbench__data-grid__light__system__clickhouse__populated__typical__active.png)
- [Redis](captures/native-workbench__data-grid__light__system__redis__populated__typical__active.png)

## Interaction evidence

- Native row selection updates the trailing value inspector.
- Native columns resize and reorder; the table scrolls and exposes a real
  context menu.
- Toolbar, menu commands, engine menu, inspector toggle, connection sheet, and
  typed destructive-review sheet operate through one presentation-only
  session.
- PostgreSQL, ClickHouse, and Redis switch the invented catalog and status
  context without network or backend access.
- Empty, loading, error, large result, long identifier, selection, pending,
  and destructive fixtures are deterministic launch routes.
- The window resizes through exact 1280×760, 1440×900, and 1720×1040 states.

## Component ownership

| Class | Visible regions |
|---|---|
| NATIVE | macOS window, system toolbar, menus/commands, search, buttons, pickers, segmented controls, sheets, split resizing, inspector presentation |
| NATIVE-COMPOSED | catalog navigation, connection form, query/result split, review list, value/structure inspector, status and filter rails, `NSViewRepresentable` boundary around native `NSTableView` |
| CUSTOM | concept mode rail, Grid Canvas floating palettes, compact document/status treatments; all remain presentation-only and use system controls internally |

The AppKit boundary exists only because SwiftUI `Table` cannot meet the
confirmed dense database-grid requirement for column reordering and resizing.
No custom data-control implementation replaces `NSTableView` behavior.

## Material ownership

| Class | Regions |
|---|---|
| CONTENT | catalog values, rows/cells, query text, results, schema/value inspector values, setup form values, status explanations, history, review details |
| FUNCTIONAL | system window/toolbar chrome, navigation chrome, inspector chrome, menus, sheets, and detached command/navigation palettes |

System toolbar, sidebar, inspector, menu, sheet, and controls own their native
material. Custom regular interactive glass appears only on the detached
functional control groups. Grid, editor, results, inspector values, setup
values, and review content remain opaque. Reduce Transparency uses an opaque
semantic fallback; Increase Contrast strengthens boundaries; Reduce Motion
removes decorative transitions.

## Evidence-based review

Scores are capture-review aids, not approval. Each category is 1–5. Weighted
total is out of 100.

| Concept | Workflow 20 | Orientation 15 | Density 15 | Native fit 15 | Keyboard 10 | Resize 10 | Safety 5 | A11y 5 | Glass 5 | Total |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Native Workbench | 5 | 5 | 5 | 5 | 5 | 4 | 4 | 4 | 5 | 96 |
| Query Studio | 4 | 3 | 4 | 4 | 5 | 4 | 3 | 4 | 5 | 79 |
| Column Observatory | 4 | 5 | 4 | 5 | 4 | 3 | 4 | 4 | 5 | 85 |
| Grid Canvas | 3 | 3 | 3 | 4 | 4 | 5 | 3 | 4 | 4 | 71 |
| Change Desk | 4 | 4 | 3 | 4 | 4 | 3 | 5 | 4 | 5 | 77 |

No concept has a gate hard failure: all preserve opaque content, dependency
isolation, real native controls, deterministic sizing, accessibility semantics,
and the production-UI boundary.

| Concept | Strengths | Weaknesses / compromise |
|---|---|---|
| Native Workbench | Best balance of orientation, density, native familiarity, and uninterrupted browse/query loop | Persistent catalog plus inspector costs width at minimum size |
| Query Studio | Strongest SQL focus and compact navigation; fast editor/result loop | Object and database context becomes less continuously visible |
| Column Observatory | Strongest source/schema orientation and sibling comparison | Four-region density compresses the work area on smaller windows |
| Grid Canvas | Calm, content-first, and most resilient when expanding | Detached navigation is less discoverable; normal states leave avoidable empty space |
| Change Desk | Strongest pending-write visibility and review continuity | Persistent ledger overweights changes during read-only browsing |

Recommendation: **Native Workbench** is the strongest base. This is only a
recommendation. The operator must choose it, choose another concept, request
changes, or explicitly name a remix.

## Accessibility and verification

The independent verifier proves an empty Swift Package/Xcode dependency graph,
builds the app, runs ten model tests, and runs five XCUITests. Runtime tests
exercise semantic accessibility auditing, named regions, table selection,
context menu actions, sheets, commands, engine switching, fixtures, and exact
minimum-window size. Contrast uses the deterministic Increase Contrast capture
because XCTest produces inconsistent sampling on opaque SwiftUI text. Known
SwiftUI/system-generated wrapper findings are narrowly excluded; application
regions and actions are asserted directly.

All 55 captures are window-ID captures from the running app. The capture script
changes no system appearance or accessibility preferences, so no system state
needs restoration.

## Known limits

- All values and actions are static presentation fixtures. No database behavior
  is implied or approved.
- Accessibility variants are explicit in-process previews; they validate the
  layout/material response without mutating system settings.
- The exhaustive scenario matrix uses Native Workbench; all concepts receive
  full surface, light/dark work-surface, and inactive-window coverage.
- Runtime review can recommend composition but cannot replace operator taste or
  approve production migration.

## Clean-room provenance

TablePro public product pages and documentation informed only broad workbench
organization: persistent database context, dense native results, query tabs,
inspectors, connection sheets, and keyboard-first flow. Apple APIs and the
installed SDK determine behavior. All names, fixtures, geometry, copy, assets,
and implementation are original TableRock work. No TablePro source, tests,
comments, bundle internals, branding, or proprietary assets were inspected or
used. The operator-rejected Sketch artifact had no influence.

## Operator decision

Native Workbench selected. See the recorded
[decision](../../design/native-macos-2026/decision-native-workbench.md).

Phase 10 refinement is complete in Design Lab only. Production UI refactor or
migration remains blocked pending the exact confirmation in the
[refined-concept gate](native-workbench-refined/README.md).
