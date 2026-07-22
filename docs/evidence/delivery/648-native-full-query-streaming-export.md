# Evidence 648: native full-query streaming export

## Claim

PostgreSQL and ClickHouse query results can start a Rust-owned asynchronous
full-result export from the native app. The bridge validates the absolute
destination and supported format, re-runs the statement through the registered
driver session, pulls 500-row pages bounded to 8 MiB and 64 KiB per cell, uses
the shared typed page projection and atomic CSV/TSV/JSON encoder, and retains
only bounded task progress.

Progress exposes rows, bytes, phase, destination, and safe summary. Cancel sets
the writer token, interrupts a pending page locally with `tokio::select!`, and
dispatches driver cancellation. Failed or cancelled exports drop the temporary
file; only completion publishes the destination. Disconnect and shutdown treat
active exports as owned operations. The native sheet exposes progress, cancel,
terminal outcome, destination name, accessibility IDs, and blocks dismissal
while active.

This is an honest partial TR-SCR-054 checkpoint. Full typed object-browse replay
and the TUI progress dialog remain open.

## Verification

```text
mise exec -- cargo test -p tablerock-ffi --test conformance --locked
mise exec -- cargo test -p tablerock-ffi --test bridge_real bridge_postgres_open_probe_fetch_shutdown --locked -- --ignored --exact --nocapture
mise exec -- cargo test -p tablerock-ffi --test bridge_real bridge_clickhouse_open_probe_fetch --locked -- --ignored --exact --nocapture
mise exec -- cargo clippy -p tablerock-core -p tablerock-files -p tablerock-cli -p tablerock-ffi --all-targets --locked -- -D warnings
mise exec -- ./scripts/build-native-app.sh --configuration Release
mise exec -- ./scripts/verify-native-result-copy.sh
```

Results: 21 conformance tests passed, including pending-page cancellation and
cleanup. PostgreSQL 18.4 exported 1,200 rows; ClickHouse 26.3 exported 501 rows.
Clippy passed. Direct Swift 6 release build passed. Native runtime gate exported
1,200 PostgreSQL rows, validated terminal progress and atomic output, and left
no temporary file.

## Clean-room provenance

TablePro public material established only the broad workflow expectation for
progress, cancellation, and visible terminal outcomes. No TablePro source,
tests, identifiers, product text, assets, colors, geometry, layout measurements,
or key bindings were read or copied. Contracts, layout, copy, and tests are
TableRock-owned and derived from existing architecture plus direct server and
runtime evidence.
