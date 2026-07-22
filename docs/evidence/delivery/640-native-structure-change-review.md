# Evidence 640: native PostgreSQL structure change review

Date: 2026-07-22

## Outcome

`TR-SCR-048` now presents native PostgreSQL structure changes through the
existing typed DDL engine contract:

- the selected cached relation is an opaque handle; Swift cannot provide a
  schema or table name;
- add/drop column, create/drop index, and add/drop constraint are a closed
  operation set, while ClickHouse and Redis remain explicitly unavailable;
- identifiers are quoted below presentation and definitions accept only the
  same bounded grammar as execution;
- staging freezes an exact statement preview, target scope, context revision,
  destructive classification, and 60-second expiry behind an opaque token;
- apply requires another explicit confirmation, consumes authority before I/O,
  cannot retry an ambiguous failure, and refreshes the target structure only
  after observed success;
- dismissal revokes unused authority; missing, expired, cross-session, and
  stale-context tokens fail closed;
- wording distinguishes PostgreSQL statement atomicity from TableRock's lack
  of automatic rollback after observed success.

Core plan construction now rejects unnamed add/drop-column operations before
they can reach an adapter. Native covers form, exact preview, review, discard,
destructive warning, confirmation, applying, success, rejection, and disabled
engine/target states.

## Verification

```text
mise exec -- cargo test -p tablerock-core ddl --locked
7 passed

mise exec -- cargo test -p tablerock-ffi --test bridge_real \
  bridge_postgres_open_probe_fetch_shutdown -- --ignored --nocapture
1 passed against PostgreSQL 18.4; exact preview, apply, observed column,
consume-once, destructive review, and revoke asserted

mise exec -- ./scripts/build-native-app.sh --configuration Release
Built native/dist/TableRock.app
```

Model and XCUITest coverage drives stage, frozen preview, second confirmation,
apply, outcome, and structure refresh. Hosted results attach to the completion
commit.

## Primary source

- PostgreSQL 18 modifying tables:
  <https://www.postgresql.org/docs/18/ddl-alter.html>

## Clean-room provenance

TablePro public material was checked only for the broad existence of
database-tool structure workflows. No source, tests, strings, assets,
screenshots, layout measurements, colors, or key bindings were copied.
TableRock's typed authority, bounds, wording, and presentation were independently
designed from repository requirements, PostgreSQL documentation, and direct
tests.
