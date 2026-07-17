# Roadmap

TableRock completed Phase 0 decision approval on 2026-07-16. The program
builds one Rust-owned PostgreSQL, ClickHouse, and Redis workbench, first as a
TermRock/Ratatui TUI and later as a native SwiftUI/AppKit macOS application.

Phase 2 owned driver execution and cancellation routing is recorded in
[`117-phase-2-operation-driver-routing.md`](docs/research/117-phase-2-operation-driver-routing.md).
The latest lookbook-only TermRock focus/table spikes and exact refreshed pin are
recorded in
[`118-termrock-focus-table-spike-update.md`](docs/research/118-termrock-focus-table-spike-update.md).
Runtime rejection now consumes session ownership and preserves cleanup evidence,
as recorded in
[`119-phase-2-runtime-rejection-ownership.md`](docs/research/119-phase-2-runtime-rejection-ownership.md).
The core-authoritative engine application-service bridge and real PostgreSQL
execution evidence are recorded in
[`120-phase-2-engine-service-bridge.md`](docs/research/120-phase-2-engine-service-bridge.md).
The latest lookbook-only TermRock textarea spike and exact refreshed pin are
recorded in
[`121-termrock-textarea-spike-update.md`](docs/research/121-termrock-textarea-spike-update.md).
TermRock sequential migration 0022, neutral paste payload adoption, and the
exact refreshed pin are recorded in
[`122-termrock-paste-payload-migration.md`](docs/research/122-termrock-paste-payload-migration.md).
Graceful and cancel-active service/runtime shutdown coordination is recorded in
[`123-phase-2-service-shutdown.md`](docs/research/123-phase-2-service-shutdown.md).
TermRock sequential migration 0023 and the exact refreshed pin are recorded in
[`124-termrock-list-multiselect-migration.md`](docs/research/124-termrock-list-multiselect-migration.md).
The shared real PostgreSQL/ClickHouse/Redis application-service harness is
recorded in
[`125-phase-2-three-engine-service-harness.md`](docs/research/125-phase-2-three-engine-service-harness.md).
Simultaneous bounded execution through that harness is recorded in
[`126-phase-2-three-engine-overlap.md`](docs/research/126-phase-2-three-engine-overlap.md).
PostgreSQL request-delivery and server-confirmed cancellation through the full
service path are recorded in
[`127-phase-2-postgresql-service-cancellation.md`](docs/research/127-phase-2-postgresql-service-cancellation.md).
The latest documentation-only TermRock semantic inventory update and exact pin
are recorded in
[`128-termrock-semantic-inventory-update.md`](docs/research/128-termrock-semantic-inventory-update.md).
ClickHouse query-ID dispatch and synchronous server-confirmed cancellation are
recorded in
[`129-phase-2-clickhouse-service-cancellation.md`](docs/research/129-phase-2-clickhouse-service-cancellation.md).
TermRock migration 0024 and its exact latest pin are adopted in
[`130-termrock-closure-runner-frame-time-migration.md`](docs/research/130-termrock-closure-runner-frame-time-migration.md).
Redis isolated blocking-command dispatch and server-confirmed unblocking are
recorded in
[`131-phase-2-redis-service-cancellation.md`](docs/research/131-phase-2-redis-service-cancellation.md).
TermRock migration 0025, one model-owned runtime keymap, and its exact latest
pin are recorded in
[`132-termrock-runtime-keymap-migration.md`](docs/research/132-termrock-runtime-keymap-migration.md).
Current-line 10,000-row streaming, first-page, throughput, page-residency, and
process-RSS budgets are recorded in
[`133-phase-2-current-line-performance-budgets.md`](docs/research/133-phase-2-current-line-performance-budgets.md).
TermRock's public Picker graduation and exact latest pin are recorded in
[`134-termrock-picker-graduation-update.md`](docs/research/134-termrock-picker-graduation-update.md).
The verified bounded offline Turso backup/restore path and independent manifest
are recorded in
[`135-phase-2-persistence-backup-restore.md`](docs/research/135-phase-2-persistence-backup-restore.md).
Verified PostgreSQL custom roots, independent server name, client identity, and
TLS cancellation on both supported lines are recorded in
[`136-phase-2-postgresql-tls-identity.md`](docs/research/136-phase-2-postgresql-tls-identity.md).
TermRock migration 0026, root-owned per-frame scoped focus registration, its
public Table graduation, and the exact latest pin are recorded in
[`137-termrock-scoped-focus-migration.md`](docs/research/137-termrock-scoped-focus-migration.md).
Redis per-command pipeline partial failures, continued execution, and
`MULTI`/`EXEC` no-rollback truth across the supported matrix are recorded in
[`138-phase-2-redis-pipeline-partial-failure.md`](docs/research/138-phase-2-redis-pipeline-partial-failure.md).
Redis missing, persistent, and finite-millisecond key TTL facts across the
supported RESP2/RESP3 matrix are recorded in
[`139-phase-2-redis-ttl-truth.md`](docs/research/139-phase-2-redis-ttl-truth.md).
TermRock's public TextArea graduation, grapheme-boundary migration 0027, and
exact latest pin are recorded in
[`140-termrock-textarea-graduation-migration.md`](docs/research/140-termrock-textarea-graduation-migration.md).
Redis HSCAN, SSCAN, and ZSCAN bounded-page behavior across both supported lines
and protocols is recorded in
[`141-phase-2-redis-collection-scans.md`](docs/research/141-phase-2-redis-collection-scans.md).
Redis live-cursor guarantees during concurrent keyspace and collection mutation
across both supported lines and protocols are recorded in
[`142-phase-2-redis-scan-mutation-races.md`](docs/research/142-phase-2-redis-scan-mutation-races.md).
Redis bounded response timing, confirmed-drop future-call reconnect, logical DB
retention, and disposable blocking-connection identity are recorded in
[`143-phase-2-redis-timeout-reconnect.md`](docs/research/143-phase-2-redis-timeout-reconnect.md).
Redis verified custom-root TLS, optional client identity, ACL authentication,
and bounded initial authentication-stop behavior across both supported lines and
protocols are recorded in
[`144-phase-2-redis-tls-authentication.md`](docs/research/144-phase-2-redis-tls-authentication.md).
Bounded, binary-safe Redis Pub/Sub isolation and truthful client-stop behavior
across the supported RESP2/RESP3 matrix are recorded in
[`145-phase-2-redis-pubsub-isolation.md`](docs/research/145-phase-2-redis-pubsub-isolation.md).
Reviewed Redis key TTL mutation and post-dispatch ambiguity evidence are
recorded in
[`146-phase-2-redis-reviewed-ttl-mutation.md`](docs/research/146-phase-2-redis-reviewed-ttl-mutation.md).
Binary-safe Redis pattern subscription paging and teardown evidence are recorded
in
[`147-phase-2-redis-pattern-subscriptions.md`](docs/research/147-phase-2-redis-pattern-subscriptions.md).
Bounded Redis Pub/Sub resubscription with explicit delivery-gap pages is
recorded in
[`148-phase-2-redis-pubsub-reconnect.md`](docs/research/148-phase-2-redis-pubsub-reconnect.md).
Redis TLS, mTLS, and ACL composition for channel and pattern Pub/Sub is recorded
in
[`149-phase-2-redis-tls-pubsub.md`](docs/research/149-phase-2-redis-tls-pubsub.md).
The restricted Redis Pub/Sub authorization boundary and current official-client
acknowledgement gap are recorded in
[`150-phase-2-redis-pubsub-acl-denial.md`](docs/research/150-phase-2-redis-pubsub-acl-denial.md).
Bounded Redis reconnect failure after live ACL credential rotation is verified
in
[`151-phase-2-redis-live-credential-revocation.md`](docs/research/151-phase-2-redis-live-credential-revocation.md).
Active Redis channel subscriptions also stop on revoked reconnect credentials as
verified in
[`152-phase-2-redis-pubsub-credential-revocation.md`](docs/research/152-phase-2-redis-pubsub-credential-revocation.md).
TLS/mTLS channel and pattern resubscription after same-endpoint server
replacement is verified in
[`153-phase-2-redis-tls-pubsub-reconnect.md`](docs/research/153-phase-2-redis-tls-pubsub-reconnect.md).
Untrusted and recredentialed TLS Pub/Sub replacement servers fail closed as
verified in
[`154-phase-2-redis-tls-pubsub-replacement-failure.md`](docs/research/154-phase-2-redis-tls-pubsub-replacement-failure.md).
PostgreSQL server-cancel versus normal-completion race outcomes are verified in
[`155-phase-2-postgresql-cancellation-completion-race.md`](docs/research/155-phase-2-postgresql-cancellation-completion-race.md).
PostgreSQL server loss before separate cancel delivery is verified in
[`156-phase-2-postgresql-cancel-transport-loss.md`](docs/research/156-phase-2-postgresql-cancel-transport-loss.md).
PostgreSQL typed parameter transport is verified in
[`157-phase-2-postgresql-typed-parameters.md`](docs/research/157-phase-2-postgresql-typed-parameters.md).
PostgreSQL typed NULL and array parameter transport is verified in
[`158-phase-2-postgresql-null-array-parameters.md`](docs/research/158-phase-2-postgresql-null-array-parameters.md).
Bounded PostgreSQL notice ownership and overflow are verified in
[`159-phase-2-postgresql-bounded-notices.md`](docs/research/159-phase-2-postgresql-bounded-notices.md).
Bounded PostgreSQL notice detail and hint fields are verified in
[`160-phase-2-postgresql-notice-detail-hint.md`](docs/research/160-phase-2-postgresql-notice-detail-hint.md).
Ordered PostgreSQL multiple-statement outcomes are verified in
[`161-phase-2-postgresql-multiple-statement-outcomes.md`](docs/research/161-phase-2-postgresql-multiple-statement-outcomes.md).

All delivery is direct, forward-only work on `main`. Never create a branch or
pull request. Each checkpoint must build, pass its evidence gate, update its
documentation, and remain honest about incomplete parity.

## Program map

| Phase | Outcome | Depends on |
|---|---|---|
| 0 | Research and architecture decisions approved (complete) | current research |
| 1 | TermRock substrate and empty TUI shell | 0 |
| 2 | Rust contracts, storage choice, and three driver spikes | 0 |
| 3 | Profiles, credentials, and connection shell | 1, 2 |
| 4 | PostgreSQL read-only vertical slice | 3, TermRock tree/grid |
| 5 | Editor, completion, grid, history, and saved-query foundation | 4, TermRock editor primitives |
| 6 | PostgreSQL editing and administration | 5 |
| 7 | ClickHouse complete engine slice | 5 |
| 8 | Redis complete engine slice | 5 |
| 9 | Cross-engine data movement and daily workflows | 6, 7, 8 |
| 10 | Scoped parity expansion | 9 |
| 11 | TUI hardening and parity release gate | 10 |
| 12 | selected macOS distribution and UniFFI proof | 11 |
| 13 | Native macOS vertical slice | 12 |
| 14 | Native workflow parity and release evidence | 13 |
| 15 | Parity closure and ongoing compatibility | 14 |

## Phase outcomes

### Phase 0 — approve research

**Status:** Complete. The operator approved all fixed decisions and authorized
Phases 0-15 on 2026-07-16. Exit evidence is recorded in
[`34-phase-0-exit-report.md`](docs/research/34-phase-0-exit-report.md).

Freeze the product boundary, clean-room process, functional-parity ledger,
TermRock ownership, TEA, Rust contract language, SecretSource model, local-only
Turso persistence, server support policy, synchronous UniFFI bridge, and direct
notarized macOS distribution. No application code or dependency is added before
this phase is approved.

### Phase 1 — TermRock and TUI foundation

**Status:** Complete. The requirement-by-requirement audit is recorded in
[`45-phase-1-exit-report.md`](docs/research/45-phase-1-exit-report.md). T0 pins and verifies the minimal TermRock consumer in
[`35-phase-1-termrock-t0.md`](docs/research/35-phase-1-termrock-t0.md). The T1
`Tree`, `Form`, and `SplitPane` checkpoints are published and repinned with evidence in
[`36-phase-1-termrock-tree.md`](docs/research/36-phase-1-termrock-tree.md) and
[`37-phase-1-termrock-form.md`](docs/research/37-phase-1-termrock-form.md), and
[`38-phase-1-termrock-split-pane.md`](docs/research/38-phase-1-termrock-split-pane.md).
Full-screen lifecycle implementation and compatibility evidence are repinned in
[`41-phase-1-terminal-lifecycle.md`](docs/research/41-phase-1-terminal-lifecycle.md).
The root TEA module boundaries, deterministic reducer, bounded subscription
declarations, responsive shell projection, focus order, minimum-size state, and
`TestBackend` evidence are implemented. The executable owns one EventStream,
maps backend input into semantic messages, renders only dirty frames, contains
panics, handles Ctrl-C/SIGTERM, rejects non-TTY execution, and has real-PTY
normal/signal restoration evidence. It instantiates the declared bounded
post-mapping root queue. Render-authorized mouse/paste/focus routing is recorded
in [`42-phase-1-render-authorized-input.md`](docs/research/42-phase-1-render-authorized-input.md).
Returned-error and panic PTY restoration evidence is recorded in
[`43-phase-1-fault-restoration.md`](docs/research/43-phase-1-fault-restoration.md).
Generic post-mapping progress coalescing and explicit overflow/resync evidence
is recorded in
[`44-phase-1-bounded-ingress.md`](docs/research/44-phase-1-bounded-ingress.md).
Domain event identity/revision mapping belongs to Phase 2.
The historical TermRock 0.8 migration is recorded in
[`49-termrock-0.8-migration.md`](docs/research/49-termrock-0.8-migration.md); the
latest 0.9 migration and exact current-main pin are recorded in
[`57-termrock-0.9-migration.md`](docs/research/57-termrock-0.9-migration.md).
The typed OSC and unknown-key migrations, with the refreshed exact main pin,
are recorded in
[`59-termrock-0.9-input-osc-migration.md`](docs/research/59-termrock-0.9-input-osc-migration.md).
TermRock's subsequent unified key-vocabulary migration and refreshed exact
main pin are recorded in
[`61-termrock-0.9-key-vocabulary-migration.md`](docs/research/61-termrock-0.9-key-vocabulary-migration.md).
The local-only Turso adoption, single-owner serialized actor, sequential
migrations, and initial real-file compatibility evidence are recorded in
[`58-phase-2-persistence-actor-foundation.md`](docs/research/58-phase-2-persistence-actor-foundation.md).
Normalized single-actor ownership and transactional interrupted-migration
recovery rules are recorded in
[`60-phase-2-persistence-ownership-recovery.md`](docs/research/60-phase-2-persistence-ownership-recovery.md).
Abrupt subprocess death without destructor/checkpoint and verified reopen are
recorded in
[`62-phase-2-persistence-crash-recovery.md`](docs/research/62-phase-2-persistence-crash-recovery.md).
Sequential saved-profile schema migration `0003` and the saved-token-only
atomic create tracer are recorded in
[`63-phase-2-saved-profile-create.md`](docs/research/63-phase-2-saved-profile-create.md).
TermRock's semantic constructible-theme migration and exact refreshed main pin
are recorded in
[`64-termrock-0.9-constructible-theme-migration.md`](docs/research/64-termrock-0.9-constructible-theme-migration.md).
Strict transactional saved-profile decoding, not-found semantics, redaction,
and reopen evidence are recorded in
[`65-phase-2-saved-profile-read.md`](docs/research/65-phase-2-saved-profile-read.md).
Atomic saved-profile replacement with exact revision compare-and-swap and
rollback evidence is recorded in
[`66-phase-2-saved-profile-replace.md`](docs/research/66-phase-2-saved-profile-replace.md).
TermRock's semantic-palette cleanup and exact refreshed main pin are recorded in
[`67-termrock-0.9-semantic-palette-migration.md`](docs/research/67-termrock-0.9-semantic-palette-migration.md).
Revision-CAS deletion and profile-owned child cleanup evidence are recorded in
[`68-phase-2-saved-profile-delete.md`](docs/research/68-phase-2-saved-profile-delete.md).
The core-owned bounded profile summary page, least-data projection, keyset
cursor, and sequential list-index migration are recorded in
[`69-phase-2-bounded-profile-list.md`](docs/research/69-phase-2-bounded-profile-list.md).
TermRock's additive alternate-preset proof and exact refreshed main pin are
recorded in
[`70-termrock-0.9-slate-preset-update.md`](docs/research/70-termrock-0.9-slate-preset-update.md).
Filter-scoped engine/favorite profile pagination and sequential migration `0005`
are recorded in
[`71-phase-2-profile-engine-favorite-filter.md`](docs/research/71-phase-2-profile-engine-favorite-filter.md).
TermRock's neutral-event migration and exact refreshed main pin are recorded in
[`72-termrock-0.9-neutral-event-migration.md`](docs/research/72-termrock-0.9-neutral-event-migration.md).
Owned cursor-scoped group/tag filters and sequential migration `0006` are
recorded in
[`73-phase-2-profile-group-tag-filter.md`](docs/research/73-phase-2-profile-group-tag-filter.md).
TermRock's canonical-module migration and exact refreshed main pin are recorded
in
[`74-termrock-0.9-canonical-module-migration.md`](docs/research/74-termrock-0.9-canonical-module-migration.md).
TermRock 0.10 trailing-metadata/multi-selection migration and exact refreshed
main pin are recorded in
[`75-termrock-0.10-metadata-selection-migration.md`](docs/research/75-termrock-0.10-metadata-selection-migration.md).
Bounded Unicode-normalized profile name/group/tag search is recorded in
[`76-phase-2-normalized-profile-search.md`](docs/research/76-phase-2-normalized-profile-search.md).
Least-privilege validated profile endpoint summary facts are recorded in
[`77-phase-2-profile-endpoint-summary.md`](docs/research/77-phase-2-profile-endpoint-summary.md).
TermRock's canonical widget-construction migration and exact refreshed main pin
are recorded in
[`78-termrock-0.10-widget-construction-migration.md`](docs/research/78-termrock-0.10-widget-construction-migration.md).
The checked row-major adapter output to immutable columnar page assembler is
recorded in
[`79-phase-2-row-major-page-assembly.md`](docs/research/79-phase-2-row-major-page-assembly.md).
TermRock's additive public-documentation hardening and exact refreshed main pin
are recorded in
[`80-termrock-0.10-documentation-hardening-update.md`](docs/research/80-termrock-0.10-documentation-hardening-update.md).
The private PostgreSQL adapter boundary, driven connection, bounded text stream,
and pinned 18.4 real-server tracer are recorded in
[`81-phase-2-postgresql-stream-foundation.md`](docs/research/81-phase-2-postgresql-stream-foundation.md).
TermRock's visible-slice scroll optimization, focus-hierarchy documentation, and
exact refreshed main pin are recorded in
[`82-termrock-0.10-visible-scroll-update.md`](docs/research/82-termrock-0.10-visible-scroll-update.md).
TermRock's sequential content-measurement revision migration `0013` and exact
refreshed main pin are recorded in
[`83-termrock-0.10-content-revision-migration.md`](docs/research/83-termrock-0.10-content-revision-migration.md).
PostgreSQL cancel-request delivery versus server-confirmed SQLSTATE `57014`,
plus post-cancel connection reuse, is recorded in
[`84-phase-2-postgresql-cancellation-truth.md`](docs/research/84-phase-2-postgresql-cancellation-truth.md).
TermRock's lookbook-only closure runner spike, forward API impact, and exact
refreshed `main` pin are recorded in
[`85-termrock-0.10-runner-spike-update.md`](docs/research/85-termrock-0.10-runner-spike-update.md).
TermRock's planning-only reconciliation and exact refreshed `main` pin are
recorded in
[`86-termrock-0.10-plan-reconciliation-update.md`](docs/research/86-termrock-0.10-plan-reconciliation-update.md).
The sole PostgreSQL extended-query typed stream and PostgreSQL 17.10/18.4
Testcontainers evidence are recorded in
[`87-phase-2-postgresql-typed-stream.md`](docs/research/87-phase-2-postgresql-typed-stream.md).
TermRock's test-only copy-on-write runtime keymap spike and exact refreshed
`main` pin are recorded in
[`88-termrock-0.10-runtime-keymap-spike-update.md`](docs/research/88-termrock-0.10-runtime-keymap-spike-update.md).
TermRock's lookbook-only generated-output hardening and exact refreshed `main`
pin are recorded in
[`89-termrock-0.10-lookbook-output-hardening-update.md`](docs/research/89-termrock-0.10-lookbook-output-hardening-update.md).
The Redis binary GET/SCAN adapter, RESP2/RESP3 and logical-database facts, and
immutable Redis 7.4.9/8.8.0 Testcontainers evidence are recorded in
[`90-phase-2-redis-binary-scan-foundation.md`](docs/research/90-phase-2-redis-binary-scan-foundation.md).
TermRock's lookbook-only interactive story controls and exact refreshed `main`
pin are recorded in
[`91-termrock-0.10-interactive-story-controls-update.md`](docs/research/91-termrock-0.10-interactive-story-controls-update.md).
TermRock's lookbook-only picker composition spike and exact refreshed `main`
pin are recorded in
[`92-termrock-0.10-picker-spike-update.md`](docs/research/92-termrock-0.10-picker-spike-update.md).
All published TermRock features and the backend-neutral input boundary are
recorded in
[`93-termrock-all-features-neutral-input-adoption.md`](docs/research/93-termrock-all-features-neutral-input-adoption.md).
TermRock's lookbook-only contract-axis story expansion and exact refreshed
`main` pin are recorded in
[`94-termrock-0.10-contract-axis-story-update.md`](docs/research/94-termrock-0.10-contract-axis-story-update.md).
The ClickHouse self-describing RowBinary stream, query-ID/compression facts,
and immutable 25.8/26.3 LTS Testcontainers evidence are recorded in
[`95-phase-2-clickhouse-rowbinary-foundation.md`](docs/research/95-phase-2-clickhouse-rowbinary-foundation.md).
TermRock's sequential scroll/hover and independent-session-options migrations,
release-flow update, and exact refreshed `main` pin are recorded in
[`96-termrock-0.10-scroll-session-migration.md`](docs/research/96-termrock-0.10-scroll-session-migration.md).
The ClickHouse precision-preserving complex scalar decoder and bounded fallback
matrix are recorded in
[`97-phase-2-clickhouse-complex-scalars.md`](docs/research/97-phase-2-clickhouse-complex-scalars.md).
The dedicated bounded structured-container value kind is recorded in
[`98-phase-2-structured-value-contract.md`](docs/research/98-phase-2-structured-value-contract.md).
Recursive ClickHouse arrays, tuples, maps, named nested records, bounded
canonical projection, and immutable 25.8/26.3 LTS Testcontainers evidence are
recorded in
[`100-phase-2-clickhouse-structured-containers.md`](docs/research/100-phase-2-clickhouse-structured-containers.md).
The explicit-open bounded result store, deterministic global LRU eviction,
revision invalidation, pinning behavior, and actual page-buffer accounting are
recorded in
[`101-phase-2-bounded-result-store.md`](docs/research/101-phase-2-bounded-result-store.md).
TermRock 0.11 migrations 0016–0017 and the exact refreshed `main` pin are
recorded in
[`102-termrock-0.11-migration.md`](docs/research/102-termrock-0.11-migration.md).
The bounded engine-native catalog forest, stable node identity, safe lazy/error
state, and revision cursor are recorded in
[`103-phase-2-catalog-snapshot.md`](docs/research/103-phase-2-catalog-snapshot.md).
The bounded typed mutation plan, truthful engine execution models, and
move-only review/authorization gate are recorded in
[`104-phase-2-mutation-plan.md`](docs/research/104-phase-2-mutation-plan.md).
TermRock migration 0018 and the exact refreshed `main` pin are recorded in
[`105-termrock-migration-0018.md`](docs/research/105-termrock-migration-0018.md).
The bounded Rust-owned single-use mutation review registry is recorded in
[`106-phase-2-mutation-review-registry.md`](docs/research/106-phase-2-mutation-review-registry.md).
The bounded operation event queue, cumulative progress coalescing, and explicit
overflow resync are recorded in
[`107-phase-2-operation-event-queue.md`](docs/research/107-phase-2-operation-event-queue.md).
TermRock migration 0019 and the exact refreshed `main` pin are recorded in
[`108-termrock-migration-0019.md`](docs/research/108-termrock-migration-0019.md).
Unified typed command/operation scope identity is recorded in
[`109-phase-2-unified-operation-scope.md`](docs/research/109-phase-2-unified-operation-scope.md).
Bounded operation ownership, parent containment, cancellation delivery,
retirement, and truthful shutdown draining are recorded in
[`110-phase-2-service-coordinator.md`](docs/research/110-phase-2-service-coordinator.md).
TermRock migration 0020, completed progress rendering, and the exact refreshed
`main` pin are recorded in
[`111-termrock-migration-0020.md`](docs/research/111-termrock-migration-0020.md).
Bounded hierarchical scope registration and authoritative command-revision
validation are recorded in
[`112-phase-2-scoped-revision-ownership.md`](docs/research/112-phase-2-scoped-revision-ownership.md).
TermRock migration 0021 and the exact refreshed `main` pin are recorded in
[`113-termrock-migration-0021.md`](docs/research/113-termrock-migration-0021.md).
Opaque bounded subscriptions, independent event fan-out, and slow-consumer
resync isolation are recorded in
[`114-phase-2-subscription-fanout.md`](docs/research/114-phase-2-subscription-fanout.md).
TermRock's published immutable frame-tick spike and the exact refreshed `main`
pin are recorded in
[`115-termrock-frame-tick-spike.md`](docs/research/115-termrock-frame-tick-spike.md).
The object-safe shared driver/page-stream seam and redacted cross-engine
contract are recorded in
[`116-phase-2-driver-adapter.md`](docs/research/116-phase-2-driver-adapter.md).

Pin an exact TermRock revision and Ratatui compatibility tuple. Build the sole
TEA Model/Message/Update/Effect/Subscription/View shell, terminal lifecycle, focus,
responsive layout, render harness, and safe shutdown. Add missing neutral
`Form`, `Tree`, `SplitPane`, and interaction primitives to TermRock `main` first,
with reusable APIs, lookbook cases, docs, tests, and Jackin compatibility.
Use Crossterm 0.29 as the sole terminal backend: one CLI EventStream for input
and TermRock's Crossterm session as the sole terminal lifecycle owner.

### Phase 2 — Rust service foundation

**Status:** In progress. The dependency-minimal authoritative ID and monotonic revision
tracer is recorded in
[`46-phase-2-core-identity.md`](docs/research/46-phase-2-core-identity.md).
The bounded owned-value and explicit per-engine capability tracer is recorded in
[`47-phase-2-value-capability-contract.md`](docs/research/47-phase-2-value-capability-contract.md).
The pre-allocation-bounded immutable columnar page tracer is recorded in
[`48-phase-2-page-contract.md`](docs/research/48-phase-2-page-contract.md).
The live-session operation lifecycle and event-identity tracer is recorded in
[`50-phase-2-operation-lifecycle.md`](docs/research/50-phase-2-operation-lifecycle.md).
The redacted failure, ambiguity, and retry-policy tracer is recorded in
[`51-phase-2-safe-diagnostics.md`](docs/research/51-phase-2-safe-diagnostics.md).
The typed scope, finite budget, and versioned command-envelope tracer is
recorded in
[`52-phase-2-command-envelope.md`](docs/research/52-phase-2-command-envelope.md).
The versioned, redacted secret-source reference tracer is recorded in
[`53-phase-2-secret-source.md`](docs/research/53-phase-2-secret-source.md).
The versioned profile property/source policy that forbids ordinary literal
secret material is recorded in
[`54-phase-2-profile-property-policy.md`](docs/research/54-phase-2-profile-property-policy.md).
The immutable connect-ready profile connection snapshot, TLS state machine, two-mode
safety policy, and finite owner limits are recorded in
[`55-phase-2-profile-snapshot.md`](docs/research/55-phase-2-profile-snapshot.md).
The baseline durable profile aggregate, saved/temporary disposition, bounded
organization/preferences, and revision replacement gate are recorded in
[`56-phase-2-profile-aggregate.md`](docs/research/56-phase-2-profile-aggregate.md).

Define owned IDs, capabilities, values, revisions, commands, events, pages,
errors, cancellation, safety, and redaction. Implement local-only Turso through
the `turso` crate on one serialized Rust async persistence actor.
Run real-server spikes for `tokio-postgres`, official `clickhouse-rs`, and
`redis-rs`, proving arbitrary values, bounded streaming, TLS, cancellation
truth, reconnect, and ambiguous-write behavior before feature claims.

PostgreSQL 17.10/18.4 now prove ordered bounded COPY OUT chunks and
prevalidated, backpressured bounded COPY IN with server-confirmed import rows.
Product file effects, cancellation/progress, partial-file policy, arbitrary
reviewed plans, and UI/UniFFI integration remain Phase 4/5/9 work.
The same pinned lines prove a dispatched write whose completion response times
out remains explicitly unknown, may later be observed exactly once, never
replays automatically, and leaves the original session usable.
An explicit transaction with deferred commit work preserves the same unknown
truth, later durable exactly-once observation, protocol drain, and no-replay
contract.
Activity-gated transport loss during deferred COMMIT also preserves unknown
truth while terminating old sessions; explicit same-directory recovery refreshes
endpoint facts, observes rollback, and never replays.
Required custom-root TLS and client identity preserve that COMMIT-loss contract
on both pinned lines: plaintext recovery is rejected, endpoint facts refresh,
mTLS is revalidated, rollback remains observable, and no replay occurs.

### Phase 3 — connection experience

Deliver searchable/organized profiles, capability-driven forms, URL and
temporary drafts, Test and Connect, General/TLS/Safety sections, 1Password
references, explicit plaintext danger, context selection, health, reconnect,
and safe versioned persistence for all three engines.

### Phase 4 — PostgreSQL read-only tracer

Deliver lazy database/schema/object catalog, object and query tabs, structure,
bounded table pages, arbitrary SQL streaming, typed values, progress, cancel,
errors, and result inspection. Add TermRock `VirtualGrid` before TableRock ships
its database-aware grid composition.

### Phase 5 — workbench foundation

Add TermRock `TextArea` and `CompletionMenu`; then deliver multiline SQL/Redis
editing, statement selection, revisioned completion, parameters, find/replace,
formatting, explain foundations, sorting/filtering, column controls, copy
formats, query files, query history, favorites, saved queries, quick switching,
and intent-only session restoration.

### Phase 6 — PostgreSQL write/admin slice

Deliver proven editability, typed value editors, inserts/updates/deletes,
staged review/undo/discard, parameterized transactional apply, conflict and
generated-value handling, foreign-key navigation, reviewed table operations,
activity/dashboard, and PostgreSQL-specific structure facts.

### Phase 7 — ClickHouse slice

Deliver databases/objects/DDL, arbitrary dynamic query results through the
official client, complex values, progress/query IDs, honest cancellation,
batch inserts, parts, explain variants, and asynchronous mutation visibility.
Never present ClickHouse mutations as transactions.

### Phase 8 — Redis slice

Deliver logical database isolation, SCAN navigation, namespaces, byte-safe
keys/values, type views, TTL, bounded server overview, command editor/completion,
pipelines, guarded type-specific edits, and honest post-dispatch cancellation.
Automatic browsing never uses `KEYS`.

### Phase 9 — daily workflows and data movement

Complete result tabs, multi-statement outcomes, saved filters/preferences,
streaming CSV/JSON/SQL import/export where meaningful, cancellation cleanup,
table operations, health/activity, robust history/search, file change handling,
restoration, cache/eviction, and cross-engine support documentation.

### Phase 10 — scoped parity expansion

Close planned later rows from the parity ledger: reviewed structure editing,
PostgreSQL backup/restore, relationship exploration, role/privilege inspection,
startup actions, optional Vim behavior, and engine-specific administration.
SSH uses one Rust `russh` tunnel adapter below the database clients. Cloud
provider proxy/identity workflows remain explicitly excluded from this program.
Features that do not apply to an engine render an explicit unsupported
capability.

### Phase 11 — TUI parity release gate

Pass reducer, widget, full-screen, PTY, real-server, failure-injection,
security, accessibility, Unicode, performance, memory, clean-room, license, and
support-matrix gates. Every in-scope parity-ledger row is implemented, explicitly
excluded, or visibly blocks the parity claim.

### Phase 12 — prove the selected native architecture

Prove the direct Developer ID/hardened-runtime/notarized release, Keychain and
1Password behavior, embedded synchronous UniFFI bridge, Swift 6 concurrency,
XCFramework packaging, cancellation, ownership, and performance. Failure blocks
native work and requires an explicit architecture revision; no secondary bridge
or distribution path is carried in the roadmap.

### Phase 13 — native vertical slice

Build the SwiftUI `App`/window/commands/settings shell, `@MainActor`
presentation store, UniFFI Rust bridge, connection experience, AppKit catalog,
query editor, large grid, result page, cancellation, and accessibility tracer.
Swift contains no database or safety behavior.

### Phase 14 — native parity and release evidence

Project all supported profiles, tabs, history, query/edit/review, engine-specific
views, import/export, files, settings, restoration, multi-window behavior,
VoiceOver, keyboard, appearance, IME, signing, hardened runtime/notarization, upgrade,
uninstall, crash recovery, and performance through the shared Rust contracts.

### Phase 15 — close and maintain parity

Audit the functional ledger, user documentation, tested server/terminal/macOS
matrix, provenance, licenses, migrations, support diagnostics, and release
artifacts. Continue compatibility work through small buildable `main` commits;
never hide an unsupported or regressed capability behind a parity claim.

Detailed deliverables and phase gates are in
[the delivery plan](docs/research/30-delivery-plan.md). The feature baseline is
[the functional parity ledger](docs/research/06-functional-parity-ledger.md).
All phases obey the forward-only dependency policy and automated freshness gate
recorded in
[`99-latest-dependency-policy.md`](docs/research/99-latest-dependency-policy.md).
