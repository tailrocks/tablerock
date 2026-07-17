# Phase 2 Immutable Catalog Snapshot

## Decision

The Rust core owns catalog vocabulary and validation. `CatalogSnapshot` is an
immutable bounded preorder forest with a scope, engine, monotonic revision,
stable opaque `CatalogNodeId`s, explicit parents/depth, and aggregate text-byte
accounting. Database client objects and borrowed strings never cross the
adapter boundary.

Node kinds remain engine-native:

- PostgreSQL: database, schema, typed object, column;
- ClickHouse: database, typed object, column;
- Redis: logical database, projected key namespace, observed server key type.

TableRock does not invent schemas for ClickHouse or relations/columns for
Redis. Redis semantic encodings such as bitmap, geospatial data, or HyperLogLog
are not fabricated from the server's key type; those belong to later bounded
value inspection.

## Lazy and failure state

Every non-leaf carries one explicit child state: unrequested, loading, loaded
complete/partial, stale, or failed. Leaves carry `NotApplicable`. Failed nodes
must carry a message-free `SafeDiagnostic` for the same engine; diagnostics on
nonfailed nodes are rejected. This prevents presentation from guessing failure
facts or retaining free-form server messages.

## Validation and revision safety

Construction rejects zero/unbounded limits, excessive nodes/depth/text,
empty names, duplicate IDs, foreign-engine kinds/types, invalid roots,
out-of-order or non-preorder parents, invalid depth, impossible hierarchy,
children on leaves, and invalid failure diagnostics. Parents must precede
children on the active preorder path, so cycles and subtree re-entry cannot be
represented.

`CatalogCursor` fixes operation scope and engine. It rejects foreign scope,
foreign engine, stale/duplicate snapshots, and revision gaps; only the immediate
next revision applies. A gap requires service resynchronization rather than a
partially reconstructed tree.

## Evidence

Public seam tests cover valid PostgreSQL, ClickHouse, and Redis trees; typed
columns; lazy/partial/stale/failed states; malformed hierarchy and preorder;
leaf/type/diagnostic ownership; node/depth/text limits; a 10,001-node synthetic
catalog; redacted debug output; and every cursor rejection class.

External concepts: engine-native catalogs, lazy tree snapshots, monotonic revision rejection
Public sources: <https://www.postgresql.org/docs/current/catalogs.html>, <https://clickhouse.com/docs/operations/system-tables>, <https://redis.io/docs/latest/commands/scan/>, <https://redis.io/docs/latest/commands/type/>, and TableRock decisions `03`/`10`/`14`/`30`/`31`
Implementation source: TableRock-owned core contract and tests
Copied code/assets/text: none
