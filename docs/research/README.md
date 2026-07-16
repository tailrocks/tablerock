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
- [Phase 1 root TEA shell evidence](39-phase-1-root-tea-shell.md)
- [Phase 1 executable loop evidence](40-phase-1-executable-loop.md)
- [Phase 1 terminal lifecycle evidence](41-phase-1-terminal-lifecycle.md)
- [Phase 1 render-authorized input evidence](42-phase-1-render-authorized-input.md)
- [Phase 1 fault restoration evidence](43-phase-1-fault-restoration.md)
- [Phase 1 bounded ingress evidence](44-phase-1-bounded-ingress.md)
- [Phase 1 exit report](45-phase-1-exit-report.md)
- [Phase 2 core identity evidence](46-phase-2-core-identity.md)
- [Phase 2 value and capability contract evidence](47-phase-2-value-capability-contract.md)
- [Phase 2 immutable page contract evidence](48-phase-2-page-contract.md)
- [TermRock 0.8 canonical API migration evidence](49-termrock-0.8-migration.md)
- [Phase 2 operation lifecycle evidence](50-phase-2-operation-lifecycle.md)
- [Phase 2 safe diagnostic evidence](51-phase-2-safe-diagnostics.md)
- [Phase 2 typed command envelope evidence](52-phase-2-command-envelope.md)
- [Phase 2 secret source evidence](53-phase-2-secret-source.md)
- [Phase 2 profile property policy evidence](54-phase-2-profile-property-policy.md)
- [Phase 2 profile connection snapshot evidence](55-phase-2-profile-snapshot.md)
- [Phase 2 profile aggregate evidence](56-phase-2-profile-aggregate.md)
- [TermRock 0.9 styled tab glyph migration evidence](57-termrock-0.9-migration.md)
- [Phase 2 persistence actor foundation evidence](58-phase-2-persistence-actor-foundation.md)
- [TermRock 0.9 input and OSC migration evidence](59-termrock-0.9-input-osc-migration.md)
- [Phase 2 persistence ownership and recovery evidence](60-phase-2-persistence-ownership-recovery.md)
- [TermRock 0.9 unified key vocabulary migration evidence](61-termrock-0.9-key-vocabulary-migration.md)
- [Phase 2 persistence crash recovery evidence](62-phase-2-persistence-crash-recovery.md)
- [Phase 2 saved-profile create evidence](63-phase-2-saved-profile-create.md)
- [TermRock 0.9 constructible theme migration evidence](64-termrock-0.9-constructible-theme-migration.md)
- [Phase 2 saved-profile read evidence](65-phase-2-saved-profile-read.md)
- [Phase 2 saved-profile replace evidence](66-phase-2-saved-profile-replace.md)
- [TermRock 0.9 semantic palette migration evidence](67-termrock-0.9-semantic-palette-migration.md)
- [Phase 2 saved-profile delete evidence](68-phase-2-saved-profile-delete.md)
- [Phase 2 bounded profile list evidence](69-phase-2-bounded-profile-list.md)
- [TermRock 0.9 slate preset update evidence](70-termrock-0.9-slate-preset-update.md)
- [Phase 2 profile engine/favorite filter evidence](71-phase-2-profile-engine-favorite-filter.md)
- [TermRock 0.9 neutral event migration evidence](72-termrock-0.9-neutral-event-migration.md)
- [Phase 2 profile group/tag filter evidence](73-phase-2-profile-group-tag-filter.md)
- [TermRock 0.9 canonical module migration evidence](74-termrock-0.9-canonical-module-migration.md)
- [TermRock 0.10 metadata and selection migration evidence](75-termrock-0.10-metadata-selection-migration.md)
- [Phase 2 normalized profile search evidence](76-phase-2-normalized-profile-search.md)
- [Phase 2 profile endpoint summary evidence](77-phase-2-profile-endpoint-summary.md)
- [TermRock 0.10 widget construction migration evidence](78-termrock-0.10-widget-construction-migration.md)
- [Phase 2 row-major page assembly evidence](79-phase-2-row-major-page-assembly.md)
- [TermRock 0.10 documentation hardening update](80-termrock-0.10-documentation-hardening-update.md)
- [Phase 2 PostgreSQL stream foundation evidence](81-phase-2-postgresql-stream-foundation.md)
- [TermRock 0.10 visible scroll update](82-termrock-0.10-visible-scroll-update.md)
- [TermRock 0.10 content revision migration](83-termrock-0.10-content-revision-migration.md)
- [Phase 2 PostgreSQL cancellation truth evidence](84-phase-2-postgresql-cancellation-truth.md)
- [TermRock 0.10 closure runner spike update](85-termrock-0.10-runner-spike-update.md)

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
