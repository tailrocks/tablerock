# Plan 012 residual — SQL UPDATE WHERE from identity columns

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `format_sql_update` takes identity_columns | done |
| WHERE id=… when identity proven | done |
| SET excludes identity columns when other cols exist | done |
| Fallback comment when no identity | done |
| Unit test | done |

## Decision

Copy-path SQL UPDATE is presentation only (not applied). When browse proved
identity columns, emit real WHERE so paste into an external client is useful.
Without identity, keep the explicit comment rather than inventing predicates.

## Evidence

```text
cargo test -p tablerock-tui --lib format_cursor_row
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for row SQL UPDATE identity WHERE
