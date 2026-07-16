# TableRock Research

Research refreshed on 2026-07-16 for a terminal-first database workbench.

## Fixed direction

- Standalone Tailrocks product, not a `jackin❯` feature.
- PostgreSQL, ClickHouse, and Redis only.
- Rust core and CLI/TUI first.
- The Elm Architecture is the sole TUI application pattern.
- TermRock is the only reusable interactive TUI component layer.
- Crossterm is the only terminal backend/input library.
- Local-only Turso through the Rust `turso` crate is the persistence store;
  `rusqlite`, `libsql`, and Turso Cloud sync are excluded.
- Native SwiftUI/AppKit embeds Rust through synchronous UniFFI.
- Native distribution is direct Developer ID with notarization/stapling.
- Official `ClickHouse/clickhouse-rs` client.
- `redis-rs/redis-rs` for Redis.
- 1Password-first connection setup; plaintext secrets remain dangerous.
- Concepts-only use of TablePro, TablePlus, and Zedis.
- Direct, forward-only delivery on `main`; no branches or pull requests.

## Single-path rule

The documents below describe one selected implementation. Comparison appears
only where needed to justify a fixed rejection, such as TEA versus Component
Architecture. An evidence failure blocks the affected phase and requires an
explicit decision revision; no parallel parser, storage, terminal backend,
widget framework, native bridge, process architecture, or distribution path is
kept in the roadmap.

## Map

- [Canonical long-running `/goal` prompt](prompt.md)
- [Vision and scope](00-vision-and-scope.md)
- [Clean-room reference policy](01-clean-room-reference.md)
- [Workflow inventory](02-workflow-inventory.md)
- [Database capability model](03-database-capabilities.md)
- [Redis reference analysis](04-redis-reference-zedis.md)
- [Product identity](05-product-identity.md)
- [Functional parity ledger](06-functional-parity-ledger.md)
- [Application pattern: TEA](07-application-pattern.md)
- [Rust core architecture](10-rust-core-architecture.md)
- [Terminal experience](11-terminal-experience.md)
- [Native macOS path](12-native-macos-path.md)
- [Primary-source platform ruling](13-platform-architecture-sources.md)
- [TermRock integration and extensions](13-termrock-integration.md)
- [Shared Rust/client contract](14-shared-client-contract.md)
- [Dependency decisions](20-dependency-evaluation.md)
- [Delivery plan](30-delivery-plan.md)
- [Fixed architecture decisions](31-fixed-decisions.md)
- [Quality and verification](32-quality-and-verification.md)
- [Main-branch delivery](33-main-branch-delivery.md)
- [Phase 0 exit report](34-phase-0-exit-report.md)
- [Phase 1 TermRock T0 evidence](35-phase-1-termrock-t0.md)
- [Phase 1 TermRock Tree evidence](36-phase-1-termrock-tree.md)
- [Phase 1 TermRock Form evidence](37-phase-1-termrock-form.md)
- [Phase 1 TermRock SplitPane evidence](38-phase-1-termrock-split-pane.md)

## Architecture headline

```text
tablerock-cli
  |- crossterm ----------> one event stream
  |- tablerock-tui ------> TEA + termrock + ratatui
  `- tablerock-engine
          |
          v
     tablerock-core

tablerock-engine/drivers/{postgres, clickhouse, redis}

TableRock.app --> SwiftUI/AppKit --> UniFFI --> tablerock-engine
```

Rust owns authoritative database state. Presentation owns focus, layout,
keyboard/mouse input, and rendering. Results cross boundaries in immutable
batches/pages rather than per cell.
