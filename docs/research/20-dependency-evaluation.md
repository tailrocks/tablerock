# Dependency Evaluation

Versions and license metadata were researched on 2026-07-15. Re-resolve version,
MSRV, features, advisories, duplicates, maintenance, and licenses in the PR that
adds each dependency.

## Baseline matrix

| Concern | Candidate | Researched version | License | Direction |
|---|---|---:|---|---|
| PostgreSQL | [`tokio-postgres`](https://github.com/rust-postgres/rust-postgres) | 0.7.18 | MIT OR Apache-2.0 | spike/adopt |
| PostgreSQL TLS | [`tokio-postgres-rustls`](https://github.com/jbg/tokio-postgres-rustls) | 0.14.0 | MIT | spike/adopt |
| ClickHouse | official [`clickhouse`](https://github.com/ClickHouse/clickhouse-rs) | 0.15.1 | MIT OR Apache-2.0 | required baseline |
| Redis | [`redis`](https://github.com/redis-rs/redis-rs) | 1.4.0 | BSD-3-Clause | required baseline |
| SQL analysis | [`sqlparser`](https://github.com/apache/datafusion-sqlparser-rs) | 0.62.0 | Apache-2.0 | baseline |
| TUI editor | [`ratatui-textarea`](https://github.com/ratatui/ratatui-textarea) | 0.9.2 | MIT | local-wrapper spike |
| TUI | future `tailrocks-tui` over [Ratatui](https://github.com/ratatui/ratatui) | release first | Apache-2.0/MIT deps | required |
| Persistence | current `turso` or focused SQLite adapter | spike | verify | one chokepoint |
| Swift binding | [`uniffi`](https://github.com/mozilla/uniffi-rs) | 0.32.0 | MPL-2.0 | deferred ruling |
| AppKit from Rust | [`objc2`](https://github.com/madsmtm/objc2) | native phase | MIT | narrow option |

## PostgreSQL

`tokio-postgres` provides explicit async connection ownership, `query_raw`
streaming, prepared metadata, and `CancelToken`. Spike arbitrary OIDs/extension
types, rustls cancellation, COPY/notices/multiple statements, portal
backpressure, large values, database pools, and failure categories. Keep all
client types behind the adapter.

Choose one rustls provider/root strategy. Never make invalid-certificate
acceptance the production default. Verify hostname, CA, client identity, and
PostgreSQL negotiation independently.

## ClickHouse

The operator requires the official client. Its typed Row APIs suit known
schemas, but a workbench needs arbitrary results. Spike `fetch_bytes()` with a
self-describing format first, then generic RowBinary if measurements require it.
Test partial errors, query IDs, server cancellation, progress permissions,
nested types, mutation polling, compression, and TLS roots.

Do not replace the client with hand-written HTTP before documenting and
assessing a narrow upstream gap.

## Redis

The operator selected [`redis-rs/redis-rs`](https://github.com/redis-rs/redis-rs).
It provides high-level and raw commands, multiplexed async connections, SCAN
options/iterators, pipelines, RESP3, cluster, Sentinel, Pub/Sub, and TLS feature
families. The first spike must prove raw bytes, all SCAN variants, logical DB
isolation, cancellation after command dispatch, blocking-command isolation,
official command metadata integration, TLS, reconnect, and a minimal feature
set.

Its BSD-3-Clause license is an explicit dependency-policy decision for this
project. Record the exact license and transitive graph in the adoption PR; do
not replace the selected client merely to narrow the allowlist.

## SQL and editor

`sqlparser` supports PostgreSQL/ClickHouse dialects, token locations, multiple
statements, AST visitors, and formatting. It is strict, not an error-tolerant
editor parser. Use token fallback, dialect-aware delimiters, bounded cursor
context, and revisioned last-known-valid AST. Reconsider tree-sitter only after
grammar/dialect/license audit and measured completion failures.

`ratatui-textarea` provides multiline buffer/cursor/selection/undo/redo/line
numbers/wrapping/scroll/search/mouse. Wrap it locally and prove shared TUI focus,
restoration, external syntax/diagnostics, completion placement, large text,
Unicode byte/character/cell mapping, and key ownership.

## Ratatui

Ratatui supplies Frame/Buffer rendering, Widget/StatefulWidget primitives,
table/list/scrollbar state, layouts, and TestBackend. The shared Tailrocks TUI
project owns higher-level focus, component state, terminal restoration,
semantic theme, lookbook, and conformance. TableRock owns database compositions.

Render only resident rows. Benchmark direct Buffer rendering only if normal
widgets miss measured scroll budgets.

## Persistence and Arrow

Choose one embedded adapter after measuring startup, migration, FTS/history,
single-process concurrency, package size, platforms, maintenance, and license.
Connection profiles remain a versioned documented config schema; result payloads
and pending edits are not persisted initially.

Do not add Arrow initially. Owned typed values plus immutable pages prove the
contract with less dependency/type/editing complexity. Reconsider for measured
million-row memory/CPU, standardized native boundary, Parquet/export, or local
vectorized analytics requirements.

## Adoption checklist

1. Record source/version/features/license/MSRV.
2. Run format/lint/test/docs plus dependency/license/advisory gates.
3. Review duplicate and unused dependencies.
4. Add an adapter; do not expose library types through `tablerock-core`.
5. Add the contract test that proves the motivating requirement.
6. Record cancellation, timeout, TLS, redaction, and failure semantics.
7. Update research when a spike rejects a baseline.
