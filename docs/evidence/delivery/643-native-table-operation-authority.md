# Evidence 643: native table-operation authority

Date: 2026-07-22

## Outcome

`TR-SCR-052` now has one Rust-owned native authority path:

- an opaque catalog node fixes the engine, object kind, database, schema, and
  table before review;
- PostgreSQL supports rename, truncate, drop, vacuum, and analyze; ClickHouse
  supports optimize; invalid engine/object/operation combinations fail closed;
- Rust creates the exact quoted preview and a 60-second, consume-once token;
- rename requires a new identifier and all applies require the exact target
  name; truncate/drop carry an additional destructive warning;
- wrong confirmation is retryable, while successful authorization consumes
  before I/O and cannot replay;
- successful operations refresh affected catalog/object state.

The existing TUI table-operation surface remains functional. Moving it onto
the same shared consume-once authority is still required for full parity.

## Verification

```text
mise exec -- cargo test -p tablerock-core ddl --locked
8 passed

mise exec -- cargo test -p tablerock-ffi --test conformance \
  table_operation_review_is_target_bound_and_wrong_confirmation_is_retryable \
  --locked
1 passed

mise exec -- cargo test -p tablerock-ffi --test bridge_real \
  bridge_postgres_open_probe_fetch_shutdown --locked -- --ignored --nocapture
1 passed against PostgreSQL 18.4; analyze, rename, and drop observed live

mise exec -- cargo test -p tablerock-ffi --test bridge_real \
  bridge_clickhouse_open_probe_fetch_shutdown --locked -- --ignored --nocapture
1 passed against ClickHouse 26.3; optimize executed live

mise exec -- cargo clippy -p tablerock-core -p tablerock-engine \
  -p tablerock-ffi --all-targets --locked -- -D warnings
passed

mise exec -- ./scripts/build-native-app.sh --configuration Release
Built native/dist/TableRock.app
```

Model and XCUITest coverage verifies frozen-target review, exact confirmation,
cancel, and successful apply states. Hosted XCTest/XCUITest and permission-
denied replay remain required before final closure.

## Clean-room provenance

TablePro public material was rechecked only to establish the broad existence
of table administration workflows. No source, tests, identifiers, strings,
assets, screenshots, layout measurements, colors, or key bindings were copied.
TableRock's operation set, authority model, SQL construction, tests, and native
presentation derive from repository requirements, official database behavior,
and direct tests.
