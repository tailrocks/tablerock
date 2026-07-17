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
- [Latest dependency policy and refresh evidence](99-latest-dependency-policy.md)
- [Phase 2 ClickHouse structured-container evidence](100-phase-2-clickhouse-structured-containers.md)
- [Phase 2 bounded result-store evidence](101-phase-2-bounded-result-store.md)
- [TermRock 0.11 migration evidence](102-termrock-0.11-migration.md)
- [Phase 2 immutable catalog snapshot evidence](103-phase-2-catalog-snapshot.md)
- [Phase 2 typed mutation-plan evidence](104-phase-2-mutation-plan.md)
- [TermRock migration 0018 adoption](105-termrock-migration-0018.md)
- [Phase 2 single-use mutation review registry](106-phase-2-mutation-review-registry.md)
- [Phase 2 bounded operation event queue](107-phase-2-operation-event-queue.md)
- [TermRock migration 0019 adoption](108-termrock-migration-0019.md)
- [Phase 2 unified operation scope](109-phase-2-unified-operation-scope.md)
- [Phase 2 application service coordinator](110-phase-2-service-coordinator.md)
- [TermRock migration 0020 adoption](111-termrock-migration-0020.md)
- [Phase 2 scoped revision ownership](112-phase-2-scoped-revision-ownership.md)
- [TermRock migration 0021 adoption](113-termrock-migration-0021.md)
- [Phase 2 bounded subscription fan-out](114-phase-2-subscription-fanout.md)
- [TermRock immutable frame-tick spike adoption](115-termrock-frame-tick-spike.md)
- [Phase 2 object-safe driver adapter](116-phase-2-driver-adapter.md)
- [Phase 2 owned driver operation runtime](117-phase-2-operation-driver-routing.md)
- [TermRock focus and table spike update](118-termrock-focus-table-spike-update.md)
- [Phase 2 runtime rejection ownership](119-phase-2-runtime-rejection-ownership.md)
- [Phase 2 engine service bridge](120-phase-2-engine-service-bridge.md)
- [TermRock textarea spike update](121-termrock-textarea-spike-update.md)
- [TermRock paste-payload migration](122-termrock-paste-payload-migration.md)
- [Phase 2 service shutdown coordination](123-phase-2-service-shutdown.md)
- [TermRock list multi-select migration](124-termrock-list-multiselect-migration.md)
- [Phase 2 three-engine service harness](125-phase-2-three-engine-service-harness.md)
- [Phase 2 three-engine overlap evidence](126-phase-2-three-engine-overlap.md)
- [Phase 2 PostgreSQL service cancellation evidence](127-phase-2-postgresql-service-cancellation.md)
- [TermRock semantic inventory update](128-termrock-semantic-inventory-update.md)
- [Phase 2 ClickHouse service cancellation evidence](129-phase-2-clickhouse-service-cancellation.md)
- [TermRock migration 0024 adoption](130-termrock-closure-runner-frame-time-migration.md)
- [Phase 2 Redis service cancellation evidence](131-phase-2-redis-service-cancellation.md)
- [TermRock migration 0025 adoption](132-termrock-runtime-keymap-migration.md)
- [Phase 2 current-line performance budgets](133-phase-2-current-line-performance-budgets.md)
- [TermRock Picker graduation update](134-termrock-picker-graduation-update.md)
- [Phase 2 persistence backup and restore evidence](135-phase-2-persistence-backup-restore.md)
- [Phase 2 PostgreSQL TLS and client identity evidence](136-phase-2-postgresql-tls-identity.md)
- [TermRock migration 0026 adoption](137-termrock-scoped-focus-migration.md)
- [Phase 2 Redis pipeline partial-failure evidence](138-phase-2-redis-pipeline-partial-failure.md)
- [Phase 2 Redis TTL truth](139-phase-2-redis-ttl-truth.md)
- [TermRock TextArea graduation and migration 0027](140-termrock-textarea-graduation-migration.md)
- [Phase 2 Redis collection SCAN evidence](141-phase-2-redis-collection-scans.md)
- [Phase 2 Redis SCAN mutation-race evidence](142-phase-2-redis-scan-mutation-races.md)
- [Phase 2 Redis timeout and reconnect evidence](143-phase-2-redis-timeout-reconnect.md)
- [Phase 2 Redis TLS and authentication evidence](144-phase-2-redis-tls-authentication.md)
- [Phase 2 Redis Pub/Sub isolation evidence](145-phase-2-redis-pubsub-isolation.md)
- [Phase 2 Redis reviewed TTL mutation evidence](146-phase-2-redis-reviewed-ttl-mutation.md)
- [Phase 2 Redis pattern subscription evidence](147-phase-2-redis-pattern-subscriptions.md)
- [Phase 2 Redis Pub/Sub reconnect evidence](148-phase-2-redis-pubsub-reconnect.md)
- [Phase 2 Redis TLS Pub/Sub composition evidence](149-phase-2-redis-tls-pubsub.md)
- [Phase 2 Redis Pub/Sub ACL denial boundary](150-phase-2-redis-pubsub-acl-denial.md)
- [Phase 2 Redis live credential revocation evidence](151-phase-2-redis-live-credential-revocation.md)
- [Phase 2 Redis Pub/Sub credential revocation evidence](152-phase-2-redis-pubsub-credential-revocation.md)
- [Phase 2 Redis TLS Pub/Sub reconnect evidence](153-phase-2-redis-tls-pubsub-reconnect.md)
- [Phase 2 Redis TLS Pub/Sub replacement failure evidence](154-phase-2-redis-tls-pubsub-replacement-failure.md)
- [Phase 2 PostgreSQL cancellation completion race evidence](155-phase-2-postgresql-cancellation-completion-race.md)
- [Phase 2 PostgreSQL cancel transport loss evidence](156-phase-2-postgresql-cancel-transport-loss.md)
- [Phase 2 PostgreSQL typed parameter evidence](157-phase-2-postgresql-typed-parameters.md)
- [Phase 2 PostgreSQL NULL and array parameter evidence](158-phase-2-postgresql-null-array-parameters.md)
- [Phase 2 PostgreSQL bounded notice evidence](159-phase-2-postgresql-bounded-notices.md)
- [Phase 2 PostgreSQL notice detail/hint evidence](160-phase-2-postgresql-notice-detail-hint.md)
- [Phase 2 PostgreSQL multiple-statement outcome evidence](161-phase-2-postgresql-multiple-statement-outcomes.md)
- [Phase 2 PostgreSQL bounded COPY streaming evidence](162-phase-2-postgresql-bounded-copy-streaming.md)
- [Phase 2 PostgreSQL ambiguous write evidence](163-phase-2-postgresql-ambiguous-write.md)
- [Phase 2 PostgreSQL ambiguous commit evidence](164-phase-2-postgresql-ambiguous-commit.md)
- [Phase 2 PostgreSQL commit transport-loss evidence](165-phase-2-postgresql-commit-transport-loss.md)
- [Phase 2 PostgreSQL mTLS commit-loss evidence](166-phase-2-postgresql-mtls-commit-loss.md)
- [Phase 2 PostgreSQL complex raw-value evidence](167-phase-2-postgresql-complex-raw-values.md)
- [Phase 2 PostgreSQL JSON projection evidence](168-phase-2-postgresql-json-projection.md)
- [TermRock 0.11 lookbook event update](169-termrock-0.11-lookbook-event-update.md)
- [Phase 2 Redis atomic revocation fixture](170-phase-2-redis-atomic-revocation-fixture.md)
- [Phase 2 Redis subscription connect policy](171-phase-2-redis-subscription-connect-policy.md)
- [Phase 2 PostgreSQL numeric decoder](172-phase-2-postgresql-numeric-decoder.md)
- [Phase 2 Redis administrative readiness budget](173-phase-2-redis-admin-readiness-budget.md)
- [Phase 2 PostgreSQL UUID decoder](174-phase-2-postgresql-uuid-decoder.md)
- [Phase 2 temporal value contract](175-phase-2-temporal-value-contract.md)
- [Phase 2 PostgreSQL temporal decoder](176-phase-2-postgresql-temporal-decoder.md)
- [Phase 2 PostgreSQL temporal completion](177-phase-2-postgresql-temporal-completion.md)
- [Phase 2 ClickHouse temporal projection](178-phase-2-clickhouse-temporal-projection.md)
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
- [TermRock 0.10 plan reconciliation update](86-termrock-0.10-plan-reconciliation-update.md)
- [Phase 2 PostgreSQL typed stream evidence](87-phase-2-postgresql-typed-stream.md)
- [TermRock 0.10 runtime keymap spike update](88-termrock-0.10-runtime-keymap-spike-update.md)
- [TermRock 0.10 lookbook output hardening update](89-termrock-0.10-lookbook-output-hardening-update.md)
- [Phase 2 Redis binary SCAN foundation evidence](90-phase-2-redis-binary-scan-foundation.md)
- [TermRock 0.10 interactive story controls update](91-termrock-0.10-interactive-story-controls-update.md)
- [TermRock 0.10 picker spike update](92-termrock-0.10-picker-spike-update.md)
- [TermRock all-features and neutral input adoption](93-termrock-all-features-neutral-input-adoption.md)
- [TermRock 0.10 contract-axis story update](94-termrock-0.10-contract-axis-story-update.md)
- [Phase 2 ClickHouse RowBinary foundation evidence](95-phase-2-clickhouse-rowbinary-foundation.md)
- [TermRock 0.10 scroll/session migration evidence](96-termrock-0.10-scroll-session-migration.md)
- [Phase 2 ClickHouse complex scalar evidence](97-phase-2-clickhouse-complex-scalars.md)
- [Phase 2 structured value contract](98-phase-2-structured-value-contract.md)

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
