# Phase 2 PostgreSQL Complex Raw-Value Evidence

## Checkpoint

The sole PostgreSQL binary `RowStream` path now proves bounded preservation for
`json`, `jsonb`, `int4range`, anonymous `record`, and a large `bytea` on
PostgreSQL 17.10 and 18.4. No database-client type crosses the adapter boundary.

This checkpoint preserves unsupported complex values; it does not claim
structured decoding. JSON, range, and composite navigation remain required.

## Decision

Unsupported complex values remain `Unknown` with their exact PostgreSQL type
name and raw binary payload. The ordinary per-cell bound applies before the
immutable page owns the bytes. Truncation retains the exact original payload
length. `bytea` remains `Binary`, including when truncated.

This keeps one stable page contract and one result path. Future canonical
decoders can replace the adapter-private classification without inventing text,
losing protocol bytes, or exposing `tokio-postgres` types.

## Real-server evidence

The fixed typed probe returns JSON, JSONB, `int4range`, anonymous `record`, and
a 16-byte `bytea`. With an eight-byte cell limit, both supported PostgreSQL
lines prove:

- JSON, JSONB, range, and record are `Unknown` with exact engine type names;
- each complex payload owns exactly eight bytes and reports an original length
  greater than eight;
- `bytea` owns eight `0xab` bytes as `Binary` and reports original length 16;
- the stream ends normally and creates no alternate text result path.

Testcontainers Rust 0.27.3 owns the official `postgres:17.10-alpine` and
`postgres:18.4-alpine` fixtures and ephemeral ports.

## Remaining bounds

The driver receives a complete PostgreSQL data-row field before TableRock can
apply the page cell bound. Strict pre-decode transport allocation for one
unbounded field remains open. Canonical numeric, temporal, UUID, array, JSON,
range, and composite decoding also remains open.

## Verification

- supported-line typed-value Testcontainers matrix: pass;
- exact kind, engine type, retained bytes, and original-length assertions: pass;
- full PostgreSQL real-server suite and workspace gates: recorded by the
  publishing commit.

Context7 was attempted first and reported its monthly quota exhausted. API
behavior was checked against pinned `tokio-postgres` 0.7.18 source and docs and
PostgreSQL primary type and wire-protocol documentation.

External concepts: PostgreSQL binary data-row fields, JSON/JSONB, ranges, composite records, bytea
Public sources: <https://docs.rs/tokio-postgres/0.7.18>, <https://www.postgresql.org/docs/current/protocol-message-formats.html>, <https://www.postgresql.org/docs/current/datatype.html>
Implementation source: TableRock-owned adapter and independent Testcontainers fixtures
Copied code/assets/text: none
