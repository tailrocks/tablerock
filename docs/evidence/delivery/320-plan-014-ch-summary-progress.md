# Plan 014 residual — ClickHouse summary progress on status bar

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Capture `X-ClickHouse-Summary` on RowBinary stream | done |
| `DriverPageStream::progress_label` | done |
| `format_clickhouse_progress` pure formatter | done |
| GridPage `server_progress` → status line | done |
| First-page CLI pump stamps progress | done |
| Unit: formatter + status line | done |

## Decision

clickhouse-rs `fetch_bytes` exposes `BytesCursor::summary()` from the
`X-ClickHouse-Summary` HTTP header once headers arrive. Values may be partial
without `wait_end_of_query=1` — status text is honest, not a live percent
gauge. Continuous mid-stream `X-ClickHouse-Progress` events are not available
on the RowBinary cursor path; this residual surfaces summary progress, not
a second transport.

## Evidence

```text
cargo test -p tablerock-engine --lib format_clickhouse_progress
cargo test -p tablerock-tui --lib status_line_includes_query_id
cargo test -p tablerock-tui --lib grid_page
cargo check -p tablerock-cli
```

## Remaining work

- Custom CA / mTLS via `with_http_client` fixture matrix
