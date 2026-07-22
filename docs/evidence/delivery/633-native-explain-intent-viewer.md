# Evidence 633: native Explain intent and viewer

Date: 2026-07-22

## Outcome

Native Query commands now expose Explain Query. Swift submits an `explain`
intent with unchanged editor text. Rust owns engine behavior:

- PostgreSQL prefixes `EXPLAIN (FORMAT TEXT)`;
- ClickHouse prefixes plain `EXPLAIN`;
- an existing Explain statement is not double-prefixed;
- `explain` must be a complete keyword, so identifiers such as
  `explainable_table` cannot bypass wrapping;
- Redis and empty statements fail closed with typed errors;
- no path adds `ANALYZE`, which could execute the underlying statement.

Returned first-column plan lines remain in the tab's bounded result table and
open a selectable monospaced viewer with copy and close controls.

## Verification

```text
mise exec -- cargo test -p tablerock-ffi --test facade explain_intent_builds_safe_postgresql_statement
1 passed

mise exec -- cargo clippy -p tablerock-ffi --all-targets -- -D warnings
green

cd native && rtk swift build -c release
ok (build complete)

rtk git diff --check
exit 0
```

The facade test inspects the concrete driver request and proves Rust generated
the PostgreSQL statement. XCUITest connects through the shipped surface,
activates `Command-Shift-E`, inspects returned plan text, and verifies Copy is
available. Hosted Xcode execution and live PostgreSQL/ClickHouse UI replay
remain required after push.

## Primary behavior sources

- PostgreSQL current EXPLAIN documentation:
  <https://www.postgresql.org/docs/current/sql-explain.html>
- ClickHouse official client behavior and real-server Explain tests already
  owned by `crates/tablerock-engine/src/clickhouse.rs` and
  `crates/tablerock-engine/tests/clickhouse_real.rs`.

## Clean-room provenance

TablePro public SQL-editor documentation was reviewed only to confirm Explain
as a broad query-workbench workflow. No source, tests, strings, assets,
geometry, measurements, colors, layout, or key bindings were copied.
TableRock's bridge intent, controls, identifiers, viewer, and tests are
independently defined.
