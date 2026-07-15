# Workflow Inventory

These workflows are independent TableRock designs based on common database
needs and terminal constraints. Sketches show information hierarchy, not copied
screen geometry.

## Connection list

The first TUI screen lists profiles with name, engine, target, state, safety,
and secret-source warning. Visible actions provide Connect, New connection,
Edit, Test, Duplicate, and Remove. Loading, empty, failed, connecting,
connected, reconnecting, and disconnected states are explicit in text and not
color alone.

## Connection editor

The first control is **Connection source**:

- **1Password item (recommended)**;
- **Manual fields**.

Selecting a 1Password item opens metadata-only account, vault, item, and field
browsing followed by a property mapping:

```text
CONNECTION PROPERTY    1PASSWORD FIELD
Host                   hostname
Port                   port
Database / index       database
Username               username
Password               password (concealed)
CA certificate         not mapped
```

The picker never resolves values. Label/type matching may suggest mappings,
but required mappings need operator review. Persist canonical UUID-form
`op://` references and a pinned account per field. Resolve only mapped fields
during Test/Connect.

Form sections:

- **General:** engine, name, host, port, default database/index, user, sources;
- **TLS:** verify policy, roots/CA, and client identity when supported;
- **Safety:** Read only, Confirm writes, Unrestricted, timeouts, result caps;
- **Advanced:** only evidence-backed engine-specific settings.

Manual secret choices are 1Password field, Prompt every time, Host environment,
and Plaintext (dangerous; local testing only). Plaintext requires an explicit
acknowledgement and remains visibly marked after save.

Test shows server identity/version, TLS result, duration, and redacted failure.
The operator can save or connect temporarily without saving.

## Workbench composition

Wide terminals show:

```text
connection/database/schema/safety context
+ catalog ----------------+ + object/query tabs -------------------------+
| lazy databases/schemas  | | grid, structure, value, or editor          |
| tables/views/keys       | | query results and action strip             |
+-------------------------+ +--------------------------------------------+
status/progress/pending changes                 focus-aware action hints
```

Narrow terminals show one full-width region at a time. The catalog becomes a
drawer/screen, object tabs stay scrollable, and table cells scroll rather than
wrap. A minimum-size screen replaces overlapping content.

## Database and schema selection

- PostgreSQL database selection changes the underlying connection/session;
  schema selection changes catalog and query context.
- ClickHouse uses per-request/default database context according to official
  client/server semantics.
- Redis selects a logical database without racing a mutable `SELECT` state
  across concurrent work; use isolated clients/connections or serialized state.

## Catalog

The left catalog lazily expands databases, schemas/namespaces, groups, and
objects. Each node has loading/stale/error state. Filtering preserves ancestor
paths. Refresh can target a subtree or connection. Redis uses SCAN cursors and
a Load more/cursor state; it never issues `KEYS` automatically.

## Tabs

Opening an object creates a preview tab. Editing, pinning, or navigating away
makes it durable. Query/command tabs own independent text, cursor, connection
context, history, results, and errors. Restore intent lazily after restart but
do not persist result payloads or pending edits initially.

## Data grid

The grid requests bounded pages around the viewport and renders only resident
rows. It supports row/cell selection, stable two-axis scrolling, explicit
loading/error/truncated states, and typed distinctions for NULL, empty, binary,
zero, false, whitespace, unknown, and truncated values.

Scrolling resident data performs no I/O. Near an unloaded range, navigation
emits a page-fetch effect and keeps stable placeholder geometry until a
revision-matched page arrives.

## SQL editor

PostgreSQL and ClickHouse tabs provide multiline editing, line numbers,
selection, undo/redo, search, syntax spans, statement boundaries, diagnostics,
and revisioned completion. Execute selection/current statement, cancel, format,
and history remain visible actions. Multiple statements create distinct result
tabs with independent summaries.

The parser must tolerate incomplete editor text through token fallback and
last-known-valid AST rules. Plain string splitting is not acceptable.

## Redis workbench

Redis receives key-native views:

- string: text, escaped, hex, and JSON inspection;
- hash: field/value grid;
- list: index/value grid with bounded ranges;
- set: members;
- sorted set: member/score;
- stream: entry ID and fields, initially read-only.

Metadata shows type, TTL, size/cardinality where known, last refresh, and stale
state. Namespace folders are clearly projections of separators such as `:`, not
real directories. Binary/undecodable keys remain reachable in a flat group.

The Redis command editor has command-aware completion, bounded raw/typed
results, timeout/cancel status, and destructive classification. Unknown
commands are treated as writes. Blocking/streaming commands are disabled unless
isolated on a disposable connection with explicit cancellation.

## Redis Overview

A bounded current `INFO` snapshot may show uptime/version/mode, memory,
connected clients, operations per second, hit/miss facts, persistence state,
and logical-database key/expiry counts. Every value has a sample time or
unavailable reason. Do not persist raw INFO, run `MONITOR`, or imply historical
monitoring.

## Editing and review

Accepting a cell/value edit changes a local mutation queue, not the database.
Pending inserts/updates/deletes have text/gutter markers. Review groups changes
by object and shows safe old/new summaries and warnings. Executable operations
remain typed/parameterized rather than using the displayed preview as input.

- PostgreSQL applies one tab change set transactionally and detects conflicts
  from stable identity plus original/version facts.
- ClickHouse distinguishes inserts from asynchronous mutations and tracks
  accepted/running/done/failed/unknown server state.
- Redis previews exact commands and TTL effects and never describes MULTI/EXEC
  as rollback-capable.

Closing, refreshing, changing context, disconnecting, or quitting with pending
edits uses one unsaved-change policy; no path silently discards them.
