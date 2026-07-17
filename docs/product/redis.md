# Redis Screens

Redis keeps its own key-native experience. It is never squeezed into a fake
relational shape, but the workbench frame — context bar, sidebar, tabs,
status — stays the same.

## Sidebar

- Logical databases (`db0`, `db1`, …) replace schemas; switching uses
  isolated connection state so concurrent tabs never race a shared `SELECT`.
- Inside a database, keys group into projected namespaces split on `:`
  separators. Namespaces are projections, not real directories; binary or
  undecodable keys stay reachable in a flat group.
- Browsing uses SCAN cursors with an explicit **load more** state. It never
  issues `KEYS`. Totals are unknown by design while scanning.
- A filter narrows the SCAN pattern; it never claims exact counts.

## Key tabs

Opening a key shows its type-specific view plus metadata: type, TTL (with
missing/persistent/finite truth), size or cardinality where known, last
refresh, stale state.

| Type | View |
|---|---|
| string | text, escaped, hex, and JSON inspection |
| hash | field/value grid |
| list | index/value grid with bounded ranges |
| set | member grid |
| sorted set | member/score grid |
| stream | entry IDs and fields; read-only in the first program |

Values load bounded pages; large values state their truncation.

## Command editor

A command tab mirrors the SQL editor: multiline input, command-aware
completion, execution, typed results, elapsed time, cancel with honest
post-dispatch semantics. Unknown commands classify as writes; blocking
commands are denied or isolated on a disposable connection with explicit
cancellation. MULTI/EXEC is presented as grouping, never as rollback.

## Editing

Type-specific staged edits follow [Editing and review](editing.md): string
values, hash fields, set/sorted-set members, list entries, key renames, and
TTL changes stage, highlight, preview as the exact commands (including TTL
preservation or replacement), and apply sequentially. No transactional
language is used.

## Overview

A bounded current `INFO` snapshot shows uptime/version/mode, memory, clients,
ops/sec, hit/miss, persistence state, and per-database key/expiry counts —
each value with its sample time or an unavailable reason. No `MONITOR`, no
implied history.
