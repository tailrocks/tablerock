# Plan 020 — AppKit catalog and Rust-owned intent

Date: 2026-07-19  
SDK: Xcode 26.6 / macOS 26.5 SDK

## Checkpoint

The native sidebar now projects catalog snapshots through an AppKit
`NSOutlineView` in `NSScrollView`. PostgreSQL and ClickHouse rows group by
schema/database; Redis keys remain flat. The coordinator uses reusable cells,
preserves expansion and selection by stable presentation keys across immutable
snapshot updates, and exposes table/object accessibility labels.

Catalog and query-result snapshots are separate, preventing catalog refreshes
from replacing the active result grid.

The audit found engine-specific catalog SQL in Swift. The enabling condition
was removed: the FFI facade now accepts a typed `catalog` intent and owns the
PostgreSQL/ClickHouse listing statements and Redis scan selection. Swift sends
no catalog statement and contains no engine catalog logic.

## Evidence

| Gate | Observation |
|------|-------------|
| `cargo test -p tablerock-ffi` | 17 passed, 5 ignored; catalog intent accepted on all engine adapters |
| `./scripts/build-native-app.sh` | PASS with Swift 6 complete concurrency and warnings-as-errors |
| `./scripts/verify-native-behavior.sh` | query + typed catalog PASS on PostgreSQL 18.4, ClickHouse 25.8, Redis 8.0 |
| PostgreSQL catalog | `schema,table`, 68 bounded rows in fixture |
| ClickHouse catalog | `schema,table`, 150 bounded rows in fixture |
| Redis catalog | binary-safe key page, empty fixture accepted |
| source audit | Swift submits `intent: "catalog"`, `statement: nil`; no engine-specific catalog SQL |

## Bounds

This checkpoint presents the bounded top-level catalog snapshot and preserves
local outline state. It does not yet dispatch Rust catalog subtree requests on
each expansion; truly lazy hierarchy remains open. Native UI automation was
attempted against a temporary PostgreSQL fixture but macOS denied assistive
access (`osascript` error `-1719`), so Connect→outline click automation is not
claimed. Strict build and live bridge/page evidence remain green.
