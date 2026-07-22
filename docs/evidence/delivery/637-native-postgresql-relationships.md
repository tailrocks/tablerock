# Evidence 637: native PostgreSQL relationships

Date: 2026-07-22

## Outcome

`TR-SCR-045` now has a native PostgreSQL relationship browser backed by one
typed Rust driver contract:

- the engine queries inbound and outbound foreign-key column edges from
  `pg_catalog` with bound schema/relation values and a hard 512-edge cap;
- the snapshot preserves composite keys, reports truncation, and returns an
  explicit empty collection when no relationship exists;
- `SessionSlot` delegates the contract so pooled and direct sessions cannot
  diverge;
- UniFFI accepts only an opaque cached PostgreSQL relation handle and returns
  typed edges, never SQL or presentation-owned catalog interpretation;
- native exposes loading, empty, populated, truncated, self-reference,
  failure, refresh, open-related, and close behavior. Opening a related table
  resolves the exact schema parent from the current catalog and reports a
  stale/unloaded target instead of guessing.

## Verification

```text
mise exec -- cargo clippy -p tablerock-engine -p tablerock-ffi \
  --all-targets -- -D warnings
green

mise exec -- cargo nextest run -p tablerock-engine -p tablerock-ffi \
  --no-fail-fast
251 tests passed; 5 skipped before this checkpoint's final focused rerun

mise exec -- ./scripts/build-native-app.sh --configuration Release
Built native/dist/TableRock.app
```

Coverage includes real PostgreSQL inbound, outbound, composite, self-cycle,
and missing-relation snapshots; a 4,096-node non-recursive graph replay; native
typed model projection; and an XCUITest opening the browser from a selected
catalog object. The canonical hosted Xcode checkpoint is required before this
evidence becomes final.

## Primary source

- PostgreSQL 18 catalog contracts for constraints and attributes:
  <https://www.postgresql.org/docs/18/catalog-pg-constraint.html>
  and <https://www.postgresql.org/docs/18/catalog-pg-attribute.html>

## Clean-room provenance

TablePro public material was checked only for the broad existence of
relationship exploration in a database workbench. No source, tests, strings,
assets, screenshots, geometry, measurements, colors, or key bindings were
copied. The bounded typed graph, list projection, states, actions, wording,
accessibility identifiers, and tests were designed from TableRock requirements,
PostgreSQL documentation, and direct tests.
