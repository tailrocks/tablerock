# Plan 013 residual — activity permission-denied signals

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `PostgresError::PermissionDenied` (SQLSTATE 42501) | done |
| `stream_statement` / `execute_sql` classify privilege errors | done |
| `PostgresSession::signal_backend` cancel/terminate | done |
| `AdapterFailureClass::PermissionDenied` + Display | done |
| CLI projects stable cancel/terminate/activity labels | done |
| Docker: restricted role cannot signal foreign backend | done |
| TUI: BackendSignalFailed paints permission label | done |

## Decision

Activity cancel/terminate must never look like a generic query failure when
the server returns insufficient privilege. Restricted roles get an explicit
`permission denied: cannot cancel/terminate backends` label. Activity
snapshot read remains available without `pg_signal_backend`.

## Evidence

```text
cargo test -p tablerock-engine --test postgres_real activity_signal_permission_denied
cargo test -p tablerock-tui --lib backend_signal_permission
cargo check -p tablerock-cli
```

## Remaining work

- Optional calendar/JSON tree widgets
