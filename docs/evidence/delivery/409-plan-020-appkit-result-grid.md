# Plan 020 — AppKit result grid

Date: 2026-07-19  
SDK: Xcode 26.6 / macOS 26.5 SDK

## Checkpoint

The native result surface no longer builds one SwiftUI `Text` per cell.
`CatalogGrid` is now an `NSViewRepresentable` backed by `NSScrollView` and
`NSTableView`, with a data-source/delegate coordinator over one immutable
`PageV1Table` snapshot. Columns are rebuilt only when their identity set
changes; cell views are reused; row selection survives bounded snapshot
updates; native resizing, reordering, multiple selection, and alternating row
backgrounds are enabled.

Each cell exposes a column-and-row accessibility label plus its value. The
table exposes the `Query results` accessibility label.

## Evidence

| Gate | Observation |
|------|-------------|
| `./scripts/build-native-app.sh` | PASS with Swift 6 complete concurrency and warnings-as-errors |
| installed-SDK API check | `NSTableView` delegate/data source, reuse, selection, columns, and accessibility compile on macOS 26.5 SDK |
| app launch inspection | connection shell renders and remains interactive after adapter integration |
| `./scripts/verify-native-behavior.sh` | PASS on PostgreSQL 18.4, ClickHouse 25.8, Redis 8.0 |
| bridge audit | one page crosses UniFFI and decodes off-main; table rendering makes zero bridge calls |

## Bounds

The live engine matrix verifies the unchanged page snapshots consumed by the
grid, while strict compilation verifies the AppKit adapter surface. Automated
selection/scroll accessibility interaction and Instruments measurements remain
open. The catalog is still a SwiftUI list and the editor is still a SwiftUI
`TextEditor`; their required AppKit adapters are later checkpoints.
