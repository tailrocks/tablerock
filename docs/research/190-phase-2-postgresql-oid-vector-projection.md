# Phase 2 PostgreSQL OID-Vector Projection Evidence

## Checkpoint

The PostgreSQL adapter now projects special `oidvector` binary values into
bounded ordered structured OID lists.

## Decision

The canonical core form is `{"$oidvector":[<u32>,...]}`. Although PostgreSQL
serializes `oidvector` through its array binary machinery, the type has stricter
invariants than a generic array: exactly one dimension, zero lower bound, no
NULL bitmap or elements, OID element identity, and unsigned four-byte members.
TableRock validates those invariants directly and preserves the specialized
type rather than pretending it is an ordinary user array.

## Bounds and failure truth

- member count is capped at one million before member projection;
- invalid dimension/null flag/element OID/lower bound, negative count, wrong
  member length, NULL member, missing data, or trailing bytes become `Invalid`;
- over-count valid structure remains whole-value `Unknown`;
- output is bounded `Structured` with canonical original-length truth;
- OIDs and vector contents are never logged.

## Evidence

Unit tests cover ordered, empty, maximum-OID, bounded, invalid shape/header,
wrong member length, trailing, and over-count values. Testcontainers Rust
0.27.3 owns official `postgres:17.10-alpine` and `postgres:18.4-alpine`
fixtures. Both lines prove representative, empty, and unsigned-boundary vectors
with exact `oidvector` identity and complete structured values.

## Remaining work

Additional scalar families remain separate typed-value work. Snapshot
projection is now delivered by research 191.
OID-vector catalog interpretation/editing, public parameter plans and request
bounds, service/UI projection, and UniFFI remain open.

Context7 was attempted earlier in this checkpoint sequence and reported its
monthly quota exhausted. Current behavior was therefore checked against
PostgreSQL `REL_18_STABLE` `oidvectorrecv`/`oidvectorsend` primary source and
pinned `postgres-types` 0.2.14 metadata.

External concepts: PostgreSQL oidvector specialized array invariants and binary framing
Public sources: <https://github.com/postgres/postgres/blob/REL_18_STABLE/src/backend/utils/adt/oid.c>, <https://www.postgresql.org/docs/current/catalog-pg-proc.html>, <https://docs.rs/postgres-types/0.2.14/postgres_types/struct.Type.html>
Implementation source: TableRock-owned adapter and independent Testcontainers fixtures
Copied code/assets/text: none
