# Plan 020 — focused commands, toolbar, and macOS 26 target

Date: 2026-07-19  
SDK: Xcode 26.6 / macOS 26.5 SDK

## Checkpoint

The native app now publishes Run, Cancel, and Refresh actions through a typed
`FocusedValue`. The Query menu consumes the focused window's actions and uses
honest enabled states plus standard shortcuts. No global singleton or stale
window routing is introduced.

The workbench has a user-customizable toolbar with stable item identifiers for
connection state, catalog refresh, Run, and Cancel. Primary Run receives the
system prominent treatment; content surfaces remain opaque and no custom blur,
material, or toolbar background exists.

Audit found the local compiler target, SwiftPM manifest, behavior harness, and
bundle minimum still declared macOS 14 despite the fixed Tahoe baseline. All
now target macOS 26.0. The SwiftPM manifest advances to PackageDescription 6.2,
the first version exposing `.macOS(.v26)`.

## Evidence

| Gate | Observation |
|------|-------------|
| `./scripts/build-native-app.sh` | PASS with Swift 6 complete concurrency and warnings-as-errors, target `arm64-apple-macos26.0` |
| app launch inspection | Query menu and four toolbar items render; disconnected actions visibly disable |
| `swift package dump-package` | PASS; platform `macos` version `26.0`, tools 6.2 |
| bundle metadata | `LSMinimumSystemVersion` = `26.0` |
| `./scripts/verify-native-behavior.sh` | query + catalog PASS on PostgreSQL 18.4, ClickHouse 25.8, Redis 8.0 with macOS 26 target |
| source audit | Commands consume focused actions; toolbar items use stable IDs; no GCD/global action router |

## Bounds

Menu and toolbar dispatch the same model functions already covered through the
live bridge harness. macOS assistive-access denial still prevents automated UI
click traversal. Toolbar customization persistence, multi-window restoration,
appearance/accessibility matrices, and exact Liquid Glass API audit remain open.
