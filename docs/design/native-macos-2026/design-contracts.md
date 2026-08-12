# Design-lab contracts

## Ownership

`TableRockDesignLab` is a standalone Swift executable and Xcode application.
Its Swift Package target has an empty dependency list. The Xcode target has no
dependency on `TableRock`, `TableRockFeature`, `TableRockBridge`, the UniFFI
XCFramework, Rust artifacts, or production resources.

Lab code may import only Apple UI/foundation frameworks. All displayed data is
invented and immutable. View-local selection and disclosure state is transient
and does not leave the process.

## Stable comparison dimensions

```text
concept:
  native-workbench | query-studio | column-observatory |
  grid-canvas | change-desk

surface:
  connections | setup | data-grid | sql-results | change-review

appearance:
  system | light | dark

accessibility:
  system | reduce-transparency | increase-contrast | reduce-motion

engine:
  postgresql | clickhouse | redis

fixture:
  populated | empty | loading | connection-error | large-result |
  long-identifiers | selected-cell | pending-change | destructive-review

window-size:
  minimum | typical | expanded
```

Launch arguments use `--concept`, `--surface`, `--appearance`,
`--accessibility`, `--engine`, `--fixture`, `--window-size`, `--inactive`, and
`--capture`. Invalid or missing values fall back to Native Workbench, Data
Grid, system appearance, system accessibility, PostgreSQL, populated data,
and typical window size. Capture mode hides lab controls but does not change
concept content.

## Interaction contract

- A single presentation-only session owns concept, surface, engine, fixture,
  selection, inspector, sheet, and command state.
- The result grid is a narrow `NSTableView` boundary because SwiftUI `Table`
  does not provide the required column reordering and database-grid density.
  It owns native row selection, scrolling, column resizing/reordering, cell
  accessibility labels, and a context menu; it contains no backend behavior.
- Connection and destructive-review flows use real sheets. Destructive apply
  requires typed `APPLY` confirmation.
- System toolbar and menu commands expose connection, query, navigation,
  engine, inspector, and review actions. `Command-T` opens a query,
  `Shift-Command-N` opens connection setup, and `Command-Return` runs the
  static query route.
- Launch routes deterministically exercise engine, fixture, appearance,
  accessibility, activity, and window-size states without changing system
  preferences.

## Isolation proof

The repository check must fail if the lab source contains any production
module import or forbidden production/backend symbol. It must also assert the
Swift Package and generated Xcode target dependency lists remain empty, then
build and test the lab independently.

Forbidden concepts include:

```text
TableRockBridge  TableRockFeature  tablerock_ffi  UniFFI  Rust
WorkbenchPresentationStore  WorkbenchBackend  persistence    credential
URLSession       Network.framework
```

The word list is defense in depth, not the module boundary itself.

## Capture contract

- Window sizes are fixed at 1280×760 minimum, 1440×900 typical, and 1720×1040
  expanded content points; captures record actual pixel dimensions.
- One filename per concept, surface, appearance, accessibility mode, engine,
  fixture, window size, and activity state.
- The 55-capture gate matrix includes all five light surfaces per concept,
  two dark work surfaces per concept, one inactive window per concept, three
  explicit accessibility previews, two alternate sizes, eight fixture
  scenarios, and two alternate engines.
- No real connection names, credentials, SQL, values, usernames, hostnames, or
  application data may appear.
- Capture manifest records git revision, host, Xcode/SDK, launch arguments,
  dimensions, and checksum.

## Clean-room provenance

TablePro's public product pages and documentation informed only broad
workbench organization: persistent database context, dense native result
tables, query tabs, inspectors, connection sheets, and keyboard-first flow.
Apple platform components and the installed macOS SDK determine behavior and
implementation. All names, fixtures, identifiers, geometry, copy, assets, and
Swift/AppKit code are original TableRock work. No TablePro source, tests,
comments, bundle internals, branding, or proprietary assets were inspected or
used.

## Gate contract

Captures and verification open the first gate; they do not close it. Work must
stop with production UI unchanged. Operator concept selection is required,
followed by a separate refined-concept confirmation before migration.
