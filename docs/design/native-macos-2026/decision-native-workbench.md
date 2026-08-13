# Native Workbench production interface

Status: production design

Decision date: 2026-08-12  
Decision owner: operator

## Decision

TableRock's native macOS application uses the Native Workbench structure:

- persistent leading database catalog;
- compact connection and database context;
- document-style object and query tabs;
- opaque, dense data and editor workspaces;
- native result tables and an optional trailing inspector;
- continuously visible safe-mode and pending-change context.

Connections open outside database context. Connected PostgreSQL and ClickHouse
objects use the catalog, data, structure, query, result, and inspector planes.
Redis uses engine-specific key, type, TTL, scan, collection, and command
surfaces inside the same shell.

## Authority and ownership

Apple platform behavior governs native interaction and accessibility. Rust owns
connection, query, result, edit, history, safety, and redaction behavior below
the synchronous UniFFI boundary. Swift owns presentation and narrow AppKit
adapters. Production code does not reproduce database policy or truth.

Liquid Glass belongs only to functional chrome. Grids, editors, inspectors,
forms, and review bodies remain opaque. Destructive and reviewed writes remain
explicit, typed, parameterized, and consume-once below presentation.

## Clean-room provenance

TablePro public pages, documentation, and observable interface informed broad
workbench organization only. Apple conventions and TableRock requirements
govern the implementation. No TablePro source, tests, comments, bundle
internals, branding, assets, product copy, credentials, or proprietary data
were used.
