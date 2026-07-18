# Plan 020 — actor-owned bridge and cancellation

Date: 2026-07-19

## Checkpoint

The presentation model previously performed synchronous UniFFI submit, pump,
event polling, and page decoding on `MainActor`, preventing responsive native
cancellation. `BridgeClient` now solely owns the UniFFI bridge. Pump and bounded
page decoding execute off-main; the actor awaits the pump reentrantly, allowing
Cancel to dispatch independently using the published operation ID. Immutable
decoded projections return to the `@MainActor @Observable` model.

The build and behavior harnesses now enforce Swift 6 complete concurrency
checking with warnings treated as errors.

## Evidence

| Gate | Observation |
|------|-------------|
| `cargo test -p tablerock-ffi` | 16 passed, 5 ignored |
| `./scripts/build-native-app.sh` | PASS; strict Swift 6, complete concurrency, warnings-as-errors |
| `./scripts/verify-native-behavior.sh` | PASS on PostgreSQL 18.4, ClickHouse 25.8, Redis 8.0 |
| source audit | one actor owns `TableRockBridge`; no GCD; decode occurs in detached tasks |
| cancellation seam | operation ID stored before pump; Cancel enabled only while active; bridge cancel contract covered by FFI suite |

## Bounds

The live matrix proves query/page behavior after the concurrency refactor. It
does not yet automate a slow-query Cancel click through the app, so the broader
vertical-slice cancellation demo criterion remains open. AppKit controls,
accessibility, appearance, and Instruments gates also remain open.
