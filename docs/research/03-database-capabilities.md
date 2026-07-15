# Database Capability Model

A universal “execute SQL” interface would hide important safety and lifecycle
differences. TableRock exposes a small shared lifecycle plus explicit
capabilities and typed engine-specific requests.

## Capability examples

```text
CatalogDatabases
CatalogSchemas
CatalogRelations
SqlExecution
RedisCommands
Transactions
EditableRows
BatchInsert
AsyncMutations
LogicalDatabases
KeyTtl
CurrentServerOverview
ServerCancellation
```

Clients show only meaningful actions and explain unavailable ones.

## Comparison

| Concern | PostgreSQL | ClickHouse | Redis |
|---|---|---|---|
| Primary model | relational/transactional | columnar analytics | key/value data structures |
| Namespace | database + schema | database | logical DB + projected key namespace |
| Browse unit | relation rows | table rows/parts | keys and type-specific values |
| Query | SQL | ClickHouse SQL | commands |
| Streaming | row stream | official-client raw/row cursor | cursor scans and bounded commands |
| Cancellation | out-of-band cancel token | query ID/server kill where supported | client/router semantics; often cannot undo command |
| Editing | transactional parameterized mutations | inserts, async UPDATE/DELETE mutations | type-specific commands, no rollback promise |
| Safety identity | primary/unique key + original/version | engine/order/partition/mutation facts | key + observed type/TTL |

## PostgreSQL

The engine owns the `tokio-postgres` client and continuously driven connection
future. Each query owns a TableRock QueryId, optional prepared statement,
bounded result sink, timeout, and cancel token.

Database switching creates/selects an appropriate underlying connection; there
is no fake `USE` statement. Schema selection remains query/catalog context.

Initial values cover NULL, bool, integer, float, decimal, text, bytes,
date/time/timestamp, UUID, JSON/JSONB, arrays, inet, and an explicit unknown
fallback. Unknown values are inspectable but not silently editable.

Editable results initially require one base relation and stable primary/unique
identity. Joins, aggregates, and no-key results remain read-only. Apply one tab
change set in a transaction; zero/multiple affected rows are conflicts and
cause rollback.

## ClickHouse

Use the official [`ClickHouse/clickhouse-rs`](https://github.com/ClickHouse/clickhouse-rs)
client over HTTP/HTTPS. A workbench executes arbitrary SQL, so the driver spike
must use self-describing/raw result formats rather than compile-time `Row`
structs only.

Preserve nullable, low-cardinality, nested, decimal, date/time, enum, array,
map, tuple, variant, and JSON facts with a safe unknown representation.

Client response consumption stop is not automatically confirmed server
cancellation. Report requested, client-stopped, server-confirmed, and unknown
states separately. Partial rows followed by an exception remain partial/failed.

Inserts may batch. UPDATE/DELETE are asynchronous mutations, not transactions;
show identity and `system.mutations` status until done/failed/unknown.

## Redis

Use SCAN/HSCAN/SSCAN/ZSCAN cursor semantics. Keyspace changes can yield
duplicates or missing keys and no stable percentage/total; the UI must tolerate
that honestly. Preserve raw bytes and use bounded value/range retrieval.

Logical databases need connection ownership that prevents concurrent `SELECT`
races. Cluster and Sentinel are deferred because routing, logical database, and
scan semantics differ materially.

TTL changes and value replacement need explicit rules: a plain SET can remove
TTL, so review must show and preserve/change it intentionally.

Unknown commands are classified as writes. Blocking commands are denied or run
on isolated disposable connections. MULTI/EXEC groups commands but does not
provide database-style rollback after a runtime error.

## Result state

Every result tracks:

- columns and original engine types;
- loaded rows/bytes and known/unknown total;
- immutable row batches/pages;
- complete/truncated/cancel-requested/cancelled/failed state;
- sort/filter provenance;
- warnings and query/command summary;
- editability and stable identity facts;
- monotonic revision.

Old catalog, document, context, or result events are rejected by revision.
Memory/row caps are enforced below presentation.

## Support policy

Do not inherit another client's server-version claims. Promise only the pinned
integration matrix: oldest real project version plus latest stable initially,
then expand when CI evidence exists.
