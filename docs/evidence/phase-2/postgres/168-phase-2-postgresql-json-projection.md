# Phase 2 PostgreSQL JSON Projection Evidence

## Checkpoint

The sole PostgreSQL binary `RowStream` path now decodes `json` and `jsonb` into
the shared bounded `Structured` value kind on PostgreSQL 17.10 and 18.4. Driver
types remain adapter-private and immutable pages remain the only output seam.

## Decision

TableRock parses the database payload as a JSON tree and emits compact JSON with
deterministically sorted object keys. The selected `serde_json` 1.0.150
`arbitrary_precision` feature preserves number lexemes beyond machine integer
and floating-point ranges. JSONB accepts only wire version one.

Projection output uses a counting bounded writer. It retains at most the cell
limit, never splits UTF-8, and records the complete canonical projection length
even after storage saturates. It does not allocate an unbounded output string.

## Failure and allocation bounds

- malformed JSON and unsupported JSONB wire versions become bounded `Invalid`
  values with exact PostgreSQL type identity;
- payloads above 8 MiB remain bounded `Unknown` without allocating a JSON DOM;
- payloads at or below 8 MiB retain the parser's recursion protection and may
  allocate a tree bounded by that input ceiling;
- the PostgreSQL driver still receives one complete field before TableRock can
  enforce this decoder bound; strict pre-driver field allocation remains open;
- structured and truncated values remain non-editable under the core mutation
  contract.

## Evidence

Unit tests prove compact key sorting, UTF-8-safe bounded output, exact canonical
length, arbitrary-precision numbers, JSONB version rejection, malformed input,
and the pre-DOM input ceiling. Official `postgres:17.10-alpine` and
`postgres:18.4-alpine` Testcontainers fixtures prove both database types become
the same exact bounded structured projection through the real binary stream.

Range, composite, array, and temporal canonical decoding remains open. Research
172 subsequently closes numeric decoding and research 174 UUID decoding;
research 167 continues to prove raw preservation where applicable.

Context7 was attempted first and reported its monthly quota exhausted. Latest
version and feature facts were verified through Cargo registry metadata,
official `serde_json` 1.0.150 docs/source, pinned `tokio-postgres` 0.7.18 source,
and PostgreSQL primary JSON and protocol documentation.

External concepts: PostgreSQL JSON/JSONB binary fields, deterministic JSON projection, bounded serialization
Public sources: <https://docs.rs/serde_json/1.0.150>, <https://www.postgresql.org/docs/current/datatype-json.html>, <https://www.postgresql.org/docs/current/protocol-message-formats.html>
Implementation source: TableRock-owned adapter, core structured-value contract, and independent Testcontainers fixtures
Copied code/assets/text: none
