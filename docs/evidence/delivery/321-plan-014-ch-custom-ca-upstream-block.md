# Plan 014 residual — ClickHouse custom CA / mTLS upstream block

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Inspect clickhouse-rs 0.15.1 `with_http_client` surface | done |
| `HttpClient` trait visibility | **private** (`mod http_client`) |
| `RequestBody` type visibility | **private** (`mod request_body`) |
| External custom connector construction | **impossible** without forking or second transport |
| Supported TLS modes remain | `Disable`, `Require` / `RequireSystemRoots` |
| Decision recorded; residual closed as upstream-blocked | done |

## Decision

Fixed decision forbids a second ClickHouse transport. clickhouse-rs
advertises `Client::with_http_client`, but the `HttpClient` trait and the
`RequestBody` body type are **crate-private**. External crates cannot
implement `HttpClient` or build `hyper_util::Client<_, RequestBody>`.

Therefore **custom CA and mTLS via a custom HttpClient are upstream-blocked**
on clickhouse-rs 0.15.1. TableRock keeps:

- `ClickHouseTlsMode::Disable` — plain HTTP
- `ClickHouseTlsMode::Require` / `RequireSystemRoots` — HTTPS with the
  crate’s built-in rustls native-roots connector

When upstream exposes a public custom-root / mTLS builder (or public
`HttpClient` + body type), re-open this residual with a fixture matrix.

## Evidence

Inspection of `clickhouse-0.15.1`:

- `src/lib.rs`: `mod http_client;` / `mod request_body;` (not `pub mod`)
- `Client::with_http_client(client: impl HttpClient)` only accepts types
  that already implement the private trait (hyper client built inside the crate)
- Workspace pin: `clickhouse = "=0.15.1"` with `rustls-tls-native-roots`,
  `rustls-tls-ring`

```text
# No live matrix — capability is compile-time unreachable for dependents.
cargo check -p tablerock-engine
```

## Remaining work

- Track upstream clickhouse-rs public TLS material API; re-open when available
