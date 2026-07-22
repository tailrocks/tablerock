# Native PostgreSQL maintenance lifecycle

Date: 2026-07-22

## Outcome

`TR-SCR-058` now has a Rust-owned asynchronous operation lifecycle for reviewed
table operations. The consume-once authority is checked before spawning;
native receives only an opaque operation ID, polls bounded status, keeps the
sheet non-dismissible while running, and projects the terminal outcome.

Reviewed DDL currently has no adapter cancellation seam. Status therefore says
`cancellable = false`, and native explains that cancellation is unavailable.
This is deliberate cancellation truth: closing the sheet cannot abandon an
operation while implying server cancellation.

The former synchronous table-operation apply entry point is removed, leaving
start/status/dismiss as the single execution architecture. Disconnect rejects
while a table operation is running. A bounded registry evicts terminal entries
only.

## Verification

```text
mise exec -- cargo check -p tablerock-ffi --locked
mise exec -- cargo test -p tablerock-ffi --test bridge_real \
  bridge_postgres_open_probe_fetch_shutdown --locked -- --ignored --nocapture
mise exec -- bash scripts/generate-swift-bindings.sh
mise exec -- bash scripts/verify-native-maintenance.sh
swift build --package-path native -c release
```

The live PostgreSQL replay starts ANALYZE, observes a successful terminal
status, verifies cancellation is unavailable, and dismisses the terminal
record. Hosted XCTest/XCUITest permission-denied and long-running visual replay
remain required for full closure.

## Clean-room provenance

TablePro's current public documentation was searched at the start of this
screen-family checkpoint. It exposed general database connection and workbench
material but no maintenance-specific public workflow suitable for adoption.
No TablePro source, tests, identifiers, strings, assets, screenshots, layout
measurements, colors, or key bindings were read or copied. TableRock's lifecycle
derives from its own requirements, official PostgreSQL behavior, existing Rust
operation ownership, and direct tests.
