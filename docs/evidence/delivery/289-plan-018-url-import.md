# Plan 018 residual — connection URL import

Date: 2026-07-18

## What landed

### Core parser (`tablerock_core::parse_connection_url`)
- Schemes: `postgresql`/`postgres`, `clickhouse`/`http`/`https`,
  `redis`/`rediss`
- Percent-decode userinfo + path; IPv6 hosts; defaults for port/database
- Query `sslmode=require|verify-*` / scheme TLS (`https`, `rediss`)
- Reject: empty, oversized, unsupported scheme, bad encoding
- **Debug redacts password** (`[N bytes]`)

### TUI
- `ActionId::ImportUrl` on Connections/Picker/Editor
- `ConfirmDialog::ImportUrl` — paste URL → Submit
- `ConnectionFormModel::apply_connection_url` fills engine/host/port/db/user
  and sets DangerousPlaintext when password present
- Status: `URL imported — review before connect`

## Commands

```bash
cargo test -p tablerock-core connection_url
cargo test -p tablerock-tui import_url
cargo test -p tablerock-tui apply_connection
```

## Ledger

URL import row: **implemented** (was gap). Update three-state CSV.
