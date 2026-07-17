# Phase 2 PostgreSQL UUID Decoder Evidence

## Checkpoint

The sole PostgreSQL binary `RowStream` path now decodes `uuid` into canonical
bounded core `Text` on PostgreSQL 17.10 and 18.4. Column metadata retains exact
PostgreSQL `uuid` identity; no client type crosses the adapter boundary.

## Decision

PostgreSQL UUID is exactly 16 network-order bytes. TableRock projects those
bytes dependency-free into the standard 8-4-4-4-12 lowercase hexadecimal form.
UUID is a typed scalar, not a structured container and not generic binary;
presentation can use column type metadata for a UUID-specific editor.

The canonical projection is always 36 bytes. The normal cell limit stores its
prefix and records original length 36. A wire payload of any other length is a
bounded `Invalid` value with exact engine type and raw truncation truth.

## Evidence

Unit tests prove representative canonical formatting, a hyphen-boundary prefix,
exact truncation metadata, and short/long malformed payloads. Official
`postgres:17.10-alpine` and `postgres:18.4-alpine` Testcontainers fixtures prove
representative, nil, and maximum UUIDs as complete `Text`, plus the existing
eight-byte typed-page probe as a truncated canonical prefix.

Context7 was attempted first and reported its monthly quota exhausted. Wire and
canonical form were verified against PostgreSQL primary UUID documentation and
pinned tokio-postgres 0.7.18 source.

External concepts: PostgreSQL UUID binary representation and canonical text form
Public sources: <https://www.postgresql.org/docs/current/datatype-uuid.html>, <https://docs.rs/tokio-postgres/0.7.18>
Implementation source: TableRock-owned bounded adapter decoder and independent Testcontainers fixtures
Copied code/assets/text: none
