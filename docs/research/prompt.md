# Research Brief

## Mission

Design TableRock as a focused PostgreSQL, ClickHouse, and Redis workbench. Ship
a standalone Rust CLI/TUI first and a native SwiftUI/AppKit macOS application
later over the same Rust core.

## Questions

1. Which commonplace database-client workflows translate well to a terminal?
2. How should PostgreSQL, ClickHouse, and Redis differences remain explicit?
3. Which Rust crates meet streaming, cancellation, TLS, dynamic-value,
   maintenance, MSRV, and license requirements?
4. How should 1Password item fields map to connection properties without
   resolving secrets during browsing or persistence?
5. How do grids, editors, catalogs, focus, responsive layouts, and safety work
   in a terminal using the shared Tailrocks TUI crate?
6. How can a future native macOS client reuse the core without duplicating
   drivers or moving Apple objects into Rust?
7. Which phases and acceptance gates keep delivery reviewable?

## In scope

- saved and temporary profiles;
- 1Password, prompt-on-connect, host environment, and dangerous plaintext
  local-test sources;
- TLS, Test, connect/disconnect/reconnect;
- database/schema/catalog navigation;
- table/key browsing and typed values;
- PostgreSQL/ClickHouse SQL and Redis commands;
- bounded streaming results, cancellation, history, autocomplete;
- staged edits, review, conflict/safety semantics;
- bounded current Redis server overview;
- future daemon and native macOS architecture.

## Excluded from the first program

- all other databases and a third-party driver marketplace;
- SSH, Cloudflare, Cloud SQL, and other tunnels/proxies;
- AI database assistance, natural-language queries, MCP, or agent-issued writes;
- import/export, backup/restore, schema editing, and historical dashboards;
- file browser and broad server administration.

## Evidence rule

External claims use primary sources and dates. TablePro, TablePlus, and Zedis
are product-problem references only. Implementation comes from this brief,
official database semantics, selected library documentation, and direct tests.
