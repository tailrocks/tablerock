# Plan 020 — Observation model and Settings scene

Date: 2026-07-19

## Checkpoint

The native prototype violated the fixed Swift ownership contract by using
`ObservableObject`, `@Published`, `@StateObject`, and `@EnvironmentObject`.
It now uses one `@MainActor @Observable` presentation model, `@State` lifetime
ownership, typed environment injection, and `@Bindable` view bindings. The app
also exposes the required native Settings scene.

## Evidence

| Gate | Observation |
|------|-------------|
| `./scripts/build-native-app.sh` | PASS under Xcode 26.6 SDK |
| forbidden observation API grep | no legacy observation APIs in app source |
| ownership | bridge and observable snapshots remain on the `@MainActor` model |

## Remaining work

Plan 020 is not complete. The current prototype still uses SwiftUI list/grid/
text controls instead of the specified AppKit outline/table/TextKit adapters,
polls synchronously on the main actor, and lacks the required accessibility,
appearance, cancellation, and Instruments evidence. Its status is corrected to
IN PROGRESS; Plan 021 remains dependency-blocked.
