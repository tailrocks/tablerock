# Evidence 646: native streaming CSV import

Date: 2026-07-22

## Outcome

`TR-SCR-053` is now native-proven through one Rust-owned pipeline:

- preview scans up to 16 GiB/100 million rows with a 64 KiB reader, 8 MiB batch
  ceiling, 64 KiB cell ceiling, exact CSV/UTF-8 positions, and a SHA-256 content
  fingerprint;
- stage copies into an exclusive mode-0600 spool, verifies the preview hash,
  rejects drift, validates every mapped typed value with constant memory, and
  freezes target/scope/revision/bytes behind 60-second consume-once authority;
- apply re-reads only the frozen spool and builds typed 500-row/8 MiB mutation
  plans—never SQL text—from the consumed authority;
- PostgreSQL commits transactional batches and reports rollback/partial truth;
  ClickHouse reports progressive batches without rollback fiction;
- shared cancellation reaches every row through one mapped atomic authority;
  native renders determinate progress, terminal/partial/unknown states, at most
  100 safe row-number errors, and Copy Errors;
- disconnect refuses active imports; shutdown drains or cancels; reject,
  expiry, discard, terminal completion, registry eviction, and bridge teardown
  remove frozen files.

## Verification

```text
mise exec -- cargo test -p tablerock-files csv_import --locked
11 passed; includes 80,000-row fixed-memory scan, byte-bounded batches,
cancellation, and exact UTF-8 errors

mise exec -- cargo test -p tablerock-ffi --test conformance \
  csv_import_freezes_and_streams_files_larger_than_legacy_buffer --locked
1 passed; 70,000 rows and >4 MiB, 100-row preview, drift rejection, source
deletion after review, frozen streaming, bounded terminal outcome

mise exec -- cargo test -p tablerock-ffi --test bridge_real \
  bridge_postgres_open_probe_fetch_shutdown --locked -- --ignored --nocapture
1 passed against PostgreSQL 18.4; 1,200 rows across three batches and exact
server count

mise exec -- cargo test -p tablerock-ffi --test bridge_real \
  bridge_clickhouse_open_probe_fetch --locked -- --ignored --nocapture
1 passed against ClickHouse 26.3; 501 rows across two batches and exact count

mise exec -- cargo clippy -p tablerock-files -p tablerock-engine \
  -p tablerock-ffi --all-targets --locked -- -D warnings
passed

mise exec -- ./scripts/build-native-app.sh --configuration Release
Swift 6 strict concurrency and warnings-as-errors passed
```

Model coverage verifies bounded safe error copying. XCUITest drives shipped
Stage/Apply/Progress/Outcome and Stage/Apply/Progress/Cancel/Outcome controls.
Hosted XCTest/XCUITest remains part of exact-SHA closure.

## Clean-room provenance

TablePro's current public import/export documentation was checked only for the
broad workflow expectations of streaming, progress, cancellation, and explicit
terminal error policy. No source, tests, identifiers, strings, assets,
screenshots, layout measurements, colors, or key bindings were copied.
TableRock's fingerprint authority, private spool, bounds, typed batches,
engine outcomes, native UI, and tests derive from plan 016, repository
architecture, Rust/database behavior, and direct tests.
