# Evidence 636: native PostgreSQL backup and restore

Date: 2026-07-22

## Outcome

`TR-SCR-044` now has a native macOS projection over the same Rust authority as
the terminal client:

- `tablerock-tools` owns direct PATH/explicit-path discovery, version probes,
  fixed argv construction, environment-only passwords, process supervision,
  cancellation, and incomplete-dump removal;
- UniFFI binds an operation to the live PostgreSQL session endpoint and its
  transient in-memory credential, exposes only bounded semantic status, limits
  concurrent processes to four, and refuses disconnect while one is active;
- shutdown includes external tools in graceful/cancel-active accounting;
- native uses `NSSavePanel`/`NSOpenPanel`, keeps security-scoped access for the
  operation lifetime, and exposes tool, archive, content, ownership, clean
  restore, review, progress, cancel, success, and failure states;
- restore review warns that a source superuser can place executable code in an
  archive. `--clean` always composes with `--if-exists`.

The process is spawned directly. No shell parses endpoint, identity, file, or
secret input. Child output is discarded rather than logged or allowed to fill
an unread pipe.

## Verification

```text
mise exec -- cargo clippy -p tablerock-tools -p tablerock-ffi \
  -p tablerock-cli --all-targets -- -D warnings
green

mise exec -- cargo test -p tablerock-ffi --test facade \
  postgres_tool_probe_is_typed_and_kind_closed
1 passed

mise exec -- cargo nextest run -p tablerock-tools -p tablerock-cli \
  -p tablerock-ffi --locked
69 passed; 12 skipped

mise exec -- cargo test -p tablerock-ffi --test bridge_real \
  bridge_postgres_open_probe_fetch_shutdown -- --ignored --nocapture
1 passed; local pg_dump lifecycle skipped because client binary is absent

mise exec -- swift build --package-path native -c release
Build complete
```

The canonical hosted real-server lane installs PostgreSQL client tools and must
prove a non-empty archive. Hosted XCUITest must prove the native file/review
flow. Live restore and mid-process cancellation replay remain the explicit
residual before this screen can become complete.

## Primary sources

- PostgreSQL 18 `pg_dump` options and archive safety warning:
  <https://www.postgresql.org/docs/18/app-pgdump.html>
- PostgreSQL 18 `pg_restore` options and archive safety warning:
  <https://www.postgresql.org/docs/18/app-pgrestore.html>

## Clean-room provenance

TablePro public material was checked only for the broad existence of a native
database backup/restore workflow. No source, tests, strings, assets, geometry,
measurements, colors, layout, or key bindings were copied. TableRock's typed
configuration, shared Rust process authority, status vocabulary, safety copy,
accessibility identifiers, and tests were independently designed from project
requirements and PostgreSQL documentation.
