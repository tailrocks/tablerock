# Phase 2 PostgreSQL Identifier Projection Evidence

## Checkpoint

The PostgreSQL adapter now projects unsigned object, transaction, command, and
registered-object identifiers through the shared core `Unsigned` contract.

## Decision

`oid`, `xid`, `cid`, and every pinned `reg*` OID alias decode as unsigned
big-endian 32-bit values. `xid8` decodes as an unsigned big-endian 64-bit value.
This preserves the full ranges, including values above signed integer maxima.
Exact PostgreSQL type identity remains in column metadata.

Registered-object types have symbolic text input/output functions, but their
binary value is the referenced OID. TableRock does not invent or freeze a
search-path-dependent name inside the cell. Future catalog metadata and
inspectors can resolve names while retaining the numeric identity as truth.

The covered OID aliases are `regproc`, `regprocedure`, `regoper`, `regoperator`,
`regclass`, `regtype`, `regconfig`, `regdictionary`, `regnamespace`, `regrole`,
and `regcollation`.

## Bounds and failure truth

- 32-bit and 64-bit widths are exact; short or trailing payloads become
  `Invalid` with exact PostgreSQL type identity;
- the core numeric representation requires eight arena bytes; a smaller caller
  cell limit becomes bounded `Unknown` rather than a partial number;
- identifier values and resolved names are never logged.

## Evidence

Unit tests cover all pinned OID aliases, maximum unsigned 32/64-bit values,
wrong-width payloads, exact failure identity, and insufficient core bounds.
Testcontainers Rust 0.27.3 owns official `postgres:17.10-alpine` and
`postgres:18.4-alpine` fixtures. Both lines prove maximum `oid`, `xid`, and
`xid8`, a live `cid`, and all eleven registered-object aliases with exact type
metadata and numeric identity.

## Remaining work

Tuple identifiers, OID vectors, LSNs, snapshots, and additional scalar families
remain separate typed-value work. Symbolic catalog enrichment, identifier
editors, public parameter plans and request bounds, service/UI projection, and
UniFFI remain open.

Context7 was attempted first and reported its monthly quota exhausted. Current
behavior was therefore checked against PostgreSQL 18 primary object-identifier
and transaction documentation plus pinned `postgres-protocol` 0.6.12 OID and
`postgres-types` 0.2.14 type metadata source.

External concepts: PostgreSQL unsigned OIDs, OID aliases, transaction/command identifiers
Public sources: <https://www.postgresql.org/docs/current/datatype-oid.html>, <https://www.postgresql.org/docs/current/functions-info.html>, <https://docs.rs/postgres-protocol/0.6.12/postgres_protocol/types/fn.oid_from_sql.html>, <https://docs.rs/postgres-types/0.2.14/postgres_types/struct.Type.html>
Implementation source: TableRock-owned adapter and independent Testcontainers fixtures
Copied code/assets/text: none
