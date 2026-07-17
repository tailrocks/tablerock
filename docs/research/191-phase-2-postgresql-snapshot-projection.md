# Phase 2 PostgreSQL Snapshot Projection Evidence

## Checkpoint

The PostgreSQL adapter now projects `pg_snapshot` and legacy
`txid_snapshot` binary values into one bounded semantic snapshot shape.

## Decision

The canonical core form is
`{"$snapshot":{"xmin":<u64>,"xmax":<u64>,"in_progress":[<u64>,...]}}`.
Column metadata retains the exact server type. Both types intentionally share
the value shape because PostgreSQL maps their receive/send functions to the
same implementation and wire format.

## Bounds and failure truth

- the in-progress count is capped at one million before projection;
- `xmin` and `xmax` must be nonzero and `xmax >= xmin`;
- in-progress IDs must be strictly increasing and within the inclusive server
  receive bounds `xmin..=xmax`;
- negative counts, missing members, invalid bounds, unordered or duplicate
  members, out-of-range members, and trailing bytes become `Invalid`;
- over-count valid headers remain whole-value `Unknown`;
- output is bounded `Structured` with canonical original-length truth;
- snapshot contents are never logged.

## Evidence

Unit tests cover both type identities, representative and empty snapshots,
unsigned boundaries, output bounds, invalid count/bounds/order/range/framing,
and the over-count fallback. Testcontainers Rust 0.27.3 owns official
`postgres:17.10-alpine` and `postgres:18.4-alpine` fixtures. Both lines prove
the modern and legacy types with exact metadata and complete structured values.

## Remaining work

Additional scalar families, catalog interpretation/editing, public parameter
plans and request bounds, service/UI projection, and UniFFI remain open.

Context7 was attempted and reported its monthly quota exhausted. Current
behavior was therefore checked against PostgreSQL `REL_18_STABLE`
`pg_snapshot_recv`/`pg_snapshot_send`, catalog aliases for legacy snapshot I/O,
current official snapshot documentation, and pinned `postgres-types` 0.2.14
metadata.

External concepts: PostgreSQL snapshot semantics, binary framing, and legacy type aliasing
Public sources: <https://github.com/postgres/postgres/blob/REL_18_STABLE/src/backend/utils/adt/xid8funcs.c>, <https://github.com/postgres/postgres/blob/REL_18_STABLE/src/include/catalog/pg_proc.dat>, <https://www.postgresql.org/docs/current/functions-info.html#FUNCTIONS-PG-SNAPSHOT>, <https://docs.rs/postgres-types/0.2.14/postgres_types/struct.Type.html>
Implementation source: TableRock-owned adapter and independent Testcontainers fixtures
Copied code/assets/text: none
