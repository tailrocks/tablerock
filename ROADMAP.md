# Roadmap

TableRock is a research-backed proposal. Each phase is independently reviewable
and leaves the repository honest about what exists.

## Phase 0: Decisions and dependency spikes

- Close naming/legal, profile scope, storage, license policy, and server matrix.
- Establish core value/capability/result contracts and driver contract tests.
- Spike PostgreSQL, official ClickHouse, Redis, SQL parsing, editor, and storage.

## Phase 1: Profiles and connection shell

- Connection list/editor for PostgreSQL, ClickHouse, and Redis.
- 1Password item mapping, TLS, safety modes, Test, and temporary connection.

## Phase 2: PostgreSQL read-only slice

- Databases, schemas, tables/views, structure, table pages, SQL, streaming,
  cancellation, typed values, and bounded results.

## Phase 3: Grid, editor, autocomplete, and PostgreSQL editing

- Viewport grid, multiline SQL editor, completion, staged mutations, review,
  transactional apply, and conflict handling.

## Phase 4: ClickHouse slice

- Official driver, databases/objects, arbitrary dynamic results, query progress,
  cancellation, inserts, parts, and asynchronous mutation visibility.

## Phase 5: Redis slice

- Logical databases, SCAN browser, namespaces, typed values, TTL, commands,
  current server overview, and guarded type-specific edits.

## Phase 6: Daily-use hardening

- History, restoration, health/reconnect, cache budgets, performance gates,
  support matrix, documentation, telemetry, and provenance audit.

## Phase 7: Service and native macOS client

- Versioned local daemon protocol and authoritative sessions.
- SwiftUI/AppKit application over coarse Rust commands, events, and pages.

Detailed scope, PR slices, and gates are in
[the delivery plan](docs/research/30-delivery-plan.md).
