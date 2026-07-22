# Evidence 635: native PostgreSQL activity

Date: 2026-07-22

## Outcome

`TR-SCR-043` now has a native projection over one shared Rust driver contract:

- bounded `pg_stat_activity` client-backend rows (maximum 32, query preview
  maximum 80 characters);
- stable typed PID, user, application, state, and query-preview fields;
- refresh, empty, loading, error, and acknowledged/not-signalable outcomes;
- per-row Cancel Query and Terminate Session actions;
- explicit confirmation before either server signal;
- exact `cancel`/`terminate` vocabulary and positive bound PID validation below
  presentation;
- permission failures remain typed as `AdapterFailureClass::PermissionDenied`
  and cross UniFFI without server text.

TUI activity handling was migrated from locally constructed SQL to the same
`DriverSession::postgres_activity` and `signal_postgres_backend` methods.
Native and TUI can no longer drift on activity query bounds, parameter binding,
or privilege classification.

## Verification

```text
mise exec -- cargo test -p tablerock-ffi --test facade \
  postgres_activity_and_signals_use_typed_driver_contract
1 passed

mise exec -- swift build --package-path native -c release
Build complete

mise exec -- cargo check -p tablerock-ffi -p tablerock-cli
green

mise exec -- cargo test -p tablerock-engine --test postgres_real \
  activity_signal_permission_denied_for_restricted_role -- --nocapture
1 passed against PostgreSQL 18.4

mise exec -- cargo test -p tablerock-ffi --test bridge_real \
  bridge_postgres_open_probe_fetch_shutdown -- --ignored --nocapture
1 passed against PostgreSQL 18.4

mise exec -- cargo nextest run -p tablerock-engine -p tablerock-cli \
  -p tablerock-ffi --locked
292 passed; 12 skipped
```

The existing PostgreSQL 18.4 real-server permission test now invokes the shared
driver methods and proves activity remains readable while both foreign-backend
signals report `PermissionDenied`. The ignored bridge-real suite additionally
loads live activity and proves an unknown PID cannot report acknowledgement.
Canonical hosted Xcode/XCUITest and the full real-server matrix remain required
after push.

## Primary sources

- PostgreSQL current monitoring statistics documentation:
  <https://www.postgresql.org/docs/current/monitoring-stats.html#MONITORING-PG-STAT-ACTIVITY-VIEW>
- PostgreSQL current server-signaling functions:
  <https://www.postgresql.org/docs/current/functions-admin.html#FUNCTIONS-ADMIN-SIGNAL>

## Clean-room provenance

TablePro public documentation was checked only to confirm server activity as a
broad database-administration workflow. No source, tests, strings, assets,
geometry, measurements, colors, layout, or key bindings were copied. TableRock
uses independently defined typed records, Rust authority, bounded SQL, native
controls, confirmation language, accessibility IDs, and direct tests.
