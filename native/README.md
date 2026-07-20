# Native / UniFFI package

Rust facade: `crates/tablerock-ffi`  
Generated Swift: `Generated/` (committed; regenerate with script)  
Bridge regression tests: `Tests/TableRockBridgeTests`
Page decode: `Sources/TableRockBridge/PageV1.swift`

## Build Rust library

```bash
cargo build -p tablerock-ffi --release
# optional universal staticlib (no full Xcode):
./scripts/build-universal-staticlib.sh
```

## Regenerate Swift bindings

```bash
# requires uniffi-bindgen 0.32.x on PATH
./scripts/generate-swift-bindings.sh
```

## Proof harness (Command Line Tools OK)

```bash
cargo build -p tablerock-ffi --release
cd native
DYLD_LIBRARY_PATH=../target/release swift test -c release
```

The SwiftPM regression target links the real generated UniFFI bridge. It owns
named lifecycle/redaction tests and hostile PageV1 decoder boundaries; app UI
automation and live database semantics remain separate testing layers.

`TableRockFeature` owns typed startup configuration, application paths, and
presentation clock/identity ports.
Production uses `Application Support/TableRock`; explicit test launches require
an absolute `TABLEROCK_TEST_ROOT`, while legacy fixture launches automatically
use a process-local temporary root. Tests and fixture gates therefore cannot
open the developer's real profiles database. Production uses system clock/UUID
implementations; deterministic tests inject fixed time and ordered identities.
File selection and pasteboard writes likewise use application-owned ports;
tests default to unavailable capabilities unless they explicitly inject one.

## XCFramework + notarization (operator)

Requires **full Xcode.app** (not only CLT) and a **Developer ID Application**
identity + notary credentials:

```bash
./scripts/build-xcframework.sh
# then sign, notarytool submit --wait, stapler staple — see plan 019
```

Plan 020's locally runnable native vertical slice is complete. Plan 019's
Developer ID/notarization distribution gate remains blocked and is inherited by
Plan 021 release evidence; it does not prevent local development or verification.

```bash
./scripts/build-native-app.sh
open native/dist/TableRock.app
```

## Workbench query tabs

Use the plus button above the SQL editor to create up to 64 independent query
tabs. Each tab owns editor text, result pages, pagination, running/cancel state,
review outcome, errors, and bound SQL file. The tab action menu renames or
closes it; running tabs cannot close, and dirty tabs require confirmation.

Saved-profile workspaces persist selected tab, titles, text, and database
intent through Rust. Results, operation handles, and pending writes never
restore. Switching profiles clears volatile tab state before loading intent.

Double-click a PostgreSQL or ClickHouse table-like catalog object to open a
read-only preview tab. Leaving the preview or choosing Pin makes it durable for
the current connection. The same object can open more than once with independent
result/page state. Rust resolves the opaque catalog handle and renders bounded
identifier-safe browse SQL; Swift never assembles object SQL. The active Redis
logical database expands through bounded SCAN/TYPE into typed key nodes;
non-UTF-8 keys retain reversible hex identity. Redis key object tabs resolve
those opaque identities below Swift and show bounded String, Hash, List, Set,
Sorted Set, and Stream views with TTL facts. Scanned Hash, Set, and Sorted Set
views page explicitly. Key mutation and namespace/context workflows remain
later parity work.

Redis connections also expose a refreshable native Overview sheet. Rust owns
the bounded INFO request and projects version/mode, uptime, memory, clients,
operations, hit/miss, persistence, and per-database facts under one sample
timestamp. Required absent fields render an explicit unavailable reason;
Swift displays the snapshot without parsing INFO.

## Multiple windows

Open additional workbench windows with the standard macOS New Window command.
All windows share one Rust bridge but own independent connection controls, tabs,
results, and errors. macOS may group them with native window tabs. Each restored
window UUID persists only editor intent and its associated profile; reconnect is
explicit, and results, credentials, operations, and pending writes never restore.

Click any loaded result cell to open its tab-local value inspector. It preserves
Rust page metadata and shows database type, nullability, value kind, truncation,
selectable text, and raw hexadecimal bytes. Structured JSON trees and editable
typed controls remain later parity work.

Result grids copy a selected cell, selected row, or all loaded rows through the
shared Rust formatter. The native pasteboard receives plain text plus CSV, TSV,
JSON, and Markdown representations. Object tabs also offer SQL INSERT when Rust
has retained base-table identity. SQL UPDATE stays absent until stable key facts
are proven; TableRock never emits an unsafe placeholder `WHERE` clause.

Result grids also export all currently loaded rows through a native save panel
as CSV, TSV, JSON, Markdown, or identity-gated SQL INSERT. Rust owns typed
formatting and atomic replacement; Swift balances security-scoped file access.
This is bounded resident export, not yet full-result streaming export.

Writable PostgreSQL and ClickHouse object tabs expose bounded CSV import with a
native preview, editable target-column mapping, explicit Text/Integer/Float/
Boolean typing, formula-literal warning, and a consume-once reviewed apply.
Rust owns file limits, parsing, catalog target identity, typed mutation plans,
review expiry, and authorization. PostgreSQL is live-proven; broader types,
ClickHouse live apply, JSON, large-file streaming, and progress/cancel remain.

PostgreSQL and ClickHouse object tabs provide a native Data/Structure switch.
Structure uses the same bounded typed Rust snapshot as the TUI. PostgreSQL
shows columns, defaults, indexes, and constraints. ClickHouse shows columns,
defaults, comments, primary/sorting membership, engine, partition/sorting/
primary expressions, and create-query metadata. Loading, empty, and error
states remain independent per object tab. Copy DDL uses bounded Rust-owned SQL;
Swift never reconstructs identifiers or constraints from visible labels.
