# Evidence Index

Every completed roadmap checkpoint records an evidence document here: the
decision, its bounds and failure truth, the verification performed, and the
remaining work. Files keep their original sequence numbers, so each group reads
in chronological order.

How to read an evidence document:

- **Checkpoint** — what changed.
- **Decision** — the single selected approach and why.
- **Bounds and failure truth** — limits, cancellation, and error behavior.
- **Evidence** — tests and real-server fixtures that prove the claims.
- **Remaining work** — open follow-ups; nothing here is claimed done.


## Phase 0 — research decisions

- 34 — [Phase 0 exit report](phase-0/34-phase-0-exit-report.md)

## Phase 1 — TermRock substrate and TUI shell

- 35 — [Phase 1 TermRock T0 evidence](phase-1/35-phase-1-termrock-t0.md)
- 36 — [Phase 1 TermRock Tree evidence](phase-1/36-phase-1-termrock-tree.md)
- 37 — [Phase 1 TermRock Form evidence](phase-1/37-phase-1-termrock-form.md)
- 38 — [Phase 1 TermRock SplitPane evidence](phase-1/38-phase-1-termrock-split-pane.md)
- 39 — [Phase 1 root TEA shell evidence](phase-1/39-phase-1-root-tea-shell.md)
- 40 — [Phase 1 executable loop evidence](phase-1/40-phase-1-executable-loop.md)
- 41 — [Phase 1 terminal lifecycle evidence](phase-1/41-phase-1-terminal-lifecycle.md)
- 42 — [Phase 1 render-authorized input evidence](phase-1/42-phase-1-render-authorized-input.md)
- 43 — [Phase 1 fault restoration evidence](phase-1/43-phase-1-fault-restoration.md)
- 44 — [Phase 1 bounded ingress evidence](phase-1/44-phase-1-bounded-ingress.md)
- 45 — [Phase 1 exit report](phase-1/45-phase-1-exit-report.md)

## Phase 2 — core contracts and services

- 46 — [Phase 2 core identity evidence](phase-2/core/46-phase-2-core-identity.md)
- 47 — [Phase 2 value and capability contract evidence](phase-2/core/47-phase-2-value-capability-contract.md)
- 48 — [Phase 2 immutable page contract evidence](phase-2/core/48-phase-2-page-contract.md)
- 50 — [Phase 2 operation lifecycle evidence](phase-2/core/50-phase-2-operation-lifecycle.md)
- 51 — [Phase 2 safe diagnostic evidence](phase-2/core/51-phase-2-safe-diagnostics.md)
- 52 — [Phase 2 typed command envelope evidence](phase-2/core/52-phase-2-command-envelope.md)
- 53 — [Phase 2 secret source evidence](phase-2/core/53-phase-2-secret-source.md)
- 79 — [Phase 2 row-major page assembly evidence](phase-2/core/79-phase-2-row-major-page-assembly.md)
- 98 — [Phase 2 structured value contract](phase-2/core/98-phase-2-structured-value-contract.md)
- 101 — [Phase 2 bounded result-store evidence](phase-2/core/101-phase-2-bounded-result-store.md)
- 103 — [Phase 2 immutable catalog snapshot evidence](phase-2/core/103-phase-2-catalog-snapshot.md)
- 104 — [Phase 2 typed mutation-plan evidence](phase-2/core/104-phase-2-mutation-plan.md)
- 106 — [Phase 2 single-use mutation review registry](phase-2/core/106-phase-2-mutation-review-registry.md)
- 107 — [Phase 2 bounded operation event queue](phase-2/core/107-phase-2-operation-event-queue.md)
- 109 — [Phase 2 unified operation scope](phase-2/core/109-phase-2-unified-operation-scope.md)
- 110 — [Phase 2 application service coordinator](phase-2/core/110-phase-2-service-coordinator.md)
- 112 — [Phase 2 scoped revision ownership](phase-2/core/112-phase-2-scoped-revision-ownership.md)
- 114 — [Phase 2 bounded subscription fan-out](phase-2/core/114-phase-2-subscription-fanout.md)
- 116 — [Phase 2 object-safe driver adapter](phase-2/core/116-phase-2-driver-adapter.md)
- 117 — [Phase 2 owned driver operation runtime](phase-2/core/117-phase-2-operation-driver-routing.md)
- 119 — [Phase 2 runtime rejection ownership](phase-2/core/119-phase-2-runtime-rejection-ownership.md)
- 120 — [Phase 2 engine service bridge](phase-2/core/120-phase-2-engine-service-bridge.md)
- 123 — [Phase 2 service shutdown coordination](phase-2/core/123-phase-2-service-shutdown.md)
- 125 — [Phase 2 three-engine service harness](phase-2/core/125-phase-2-three-engine-service-harness.md)
- 126 — [Phase 2 three-engine overlap evidence](phase-2/core/126-phase-2-three-engine-overlap.md)
- 133 — [Phase 2 current-line performance budgets](phase-2/core/133-phase-2-current-line-performance-budgets.md)
- 175 — [Phase 2 temporal value contract](phase-2/core/175-phase-2-temporal-value-contract.md)
- 193 — [Phase 2 Execute intent and StatementText](phase-2/core/193-phase-2-execute-intent.md)
- 194 — [Phase 2 session registry and Arc runtime borrow](phase-2/core/194-phase-2-session-registry.md)
- 195 — [Phase 2 arbitrary statements and session health](phase-2/core/195-phase-2-arbitrary-statements-and-health.md)

## Phase 2 — profiles

- 54 — [Phase 2 profile property policy evidence](phase-2/profiles/54-phase-2-profile-property-policy.md)
- 55 — [Phase 2 profile connection snapshot evidence](phase-2/profiles/55-phase-2-profile-snapshot.md)
- 56 — [Phase 2 profile aggregate evidence](phase-2/profiles/56-phase-2-profile-aggregate.md)
- 63 — [Phase 2 saved-profile create evidence](phase-2/profiles/63-phase-2-saved-profile-create.md)
- 65 — [Phase 2 saved-profile read evidence](phase-2/profiles/65-phase-2-saved-profile-read.md)
- 66 — [Phase 2 saved-profile replace evidence](phase-2/profiles/66-phase-2-saved-profile-replace.md)
- 68 — [Phase 2 saved-profile delete evidence](phase-2/profiles/68-phase-2-saved-profile-delete.md)
- 69 — [Phase 2 bounded profile list evidence](phase-2/profiles/69-phase-2-bounded-profile-list.md)
- 71 — [Phase 2 profile engine/favorite filter evidence](phase-2/profiles/71-phase-2-profile-engine-favorite-filter.md)
- 73 — [Phase 2 profile group/tag filter evidence](phase-2/profiles/73-phase-2-profile-group-tag-filter.md)
- 76 — [Phase 2 normalized profile search evidence](phase-2/profiles/76-phase-2-normalized-profile-search.md)
- 77 — [Phase 2 profile endpoint summary evidence](phase-2/profiles/77-phase-2-profile-endpoint-summary.md)

## Phase 2 — persistence

- 58 — [Phase 2 persistence actor foundation evidence](phase-2/persistence/58-phase-2-persistence-actor-foundation.md)
- 60 — [Phase 2 persistence ownership and recovery evidence](phase-2/persistence/60-phase-2-persistence-ownership-recovery.md)
- 62 — [Phase 2 persistence crash recovery evidence](phase-2/persistence/62-phase-2-persistence-crash-recovery.md)
- 135 — [Phase 2 persistence backup and restore evidence](phase-2/persistence/135-phase-2-persistence-backup-restore.md)

## Phase 2 — PostgreSQL driver

- 81 — [Phase 2 PostgreSQL stream foundation evidence](phase-2/postgres/81-phase-2-postgresql-stream-foundation.md)
- 84 — [Phase 2 PostgreSQL cancellation truth evidence](phase-2/postgres/84-phase-2-postgresql-cancellation-truth.md)
- 87 — [Phase 2 PostgreSQL typed stream evidence](phase-2/postgres/87-phase-2-postgresql-typed-stream.md)
- 127 — [Phase 2 PostgreSQL service cancellation evidence](phase-2/postgres/127-phase-2-postgresql-service-cancellation.md)
- 136 — [Phase 2 PostgreSQL TLS and client identity evidence](phase-2/postgres/136-phase-2-postgresql-tls-identity.md)
- 155 — [Phase 2 PostgreSQL cancellation completion race evidence](phase-2/postgres/155-phase-2-postgresql-cancellation-completion-race.md)
- 156 — [Phase 2 PostgreSQL cancel transport loss evidence](phase-2/postgres/156-phase-2-postgresql-cancel-transport-loss.md)
- 157 — [Phase 2 PostgreSQL typed parameter evidence](phase-2/postgres/157-phase-2-postgresql-typed-parameters.md)
- 158 — [Phase 2 PostgreSQL NULL and array parameter evidence](phase-2/postgres/158-phase-2-postgresql-null-array-parameters.md)
- 159 — [Phase 2 PostgreSQL bounded notice evidence](phase-2/postgres/159-phase-2-postgresql-bounded-notices.md)
- 160 — [Phase 2 PostgreSQL notice detail/hint evidence](phase-2/postgres/160-phase-2-postgresql-notice-detail-hint.md)
- 161 — [Phase 2 PostgreSQL multiple-statement outcome evidence](phase-2/postgres/161-phase-2-postgresql-multiple-statement-outcomes.md)
- 162 — [Phase 2 PostgreSQL bounded COPY streaming evidence](phase-2/postgres/162-phase-2-postgresql-bounded-copy-streaming.md)
- 163 — [Phase 2 PostgreSQL ambiguous write evidence](phase-2/postgres/163-phase-2-postgresql-ambiguous-write.md)
- 164 — [Phase 2 PostgreSQL ambiguous commit evidence](phase-2/postgres/164-phase-2-postgresql-ambiguous-commit.md)
- 165 — [Phase 2 PostgreSQL commit transport-loss evidence](phase-2/postgres/165-phase-2-postgresql-commit-transport-loss.md)
- 166 — [Phase 2 PostgreSQL mTLS commit-loss evidence](phase-2/postgres/166-phase-2-postgresql-mtls-commit-loss.md)
- 167 — [Phase 2 PostgreSQL complex raw-value evidence](phase-2/postgres/167-phase-2-postgresql-complex-raw-values.md)
- 168 — [Phase 2 PostgreSQL JSON projection evidence](phase-2/postgres/168-phase-2-postgresql-json-projection.md)
- 172 — [Phase 2 PostgreSQL numeric decoder](phase-2/postgres/172-phase-2-postgresql-numeric-decoder.md)
- 174 — [Phase 2 PostgreSQL UUID decoder](phase-2/postgres/174-phase-2-postgresql-uuid-decoder.md)
- 176 — [Phase 2 PostgreSQL temporal decoder](phase-2/postgres/176-phase-2-postgresql-temporal-decoder.md)
- 177 — [Phase 2 PostgreSQL temporal completion](phase-2/postgres/177-phase-2-postgresql-temporal-completion.md)
- 179 — [Phase 2 PostgreSQL array projection](phase-2/postgres/179-phase-2-postgresql-array-projection.md)
- 180 — [Phase 2 PostgreSQL range projection](phase-2/postgres/180-phase-2-postgresql-range-projection.md)
- 181 — [Phase 2 PostgreSQL multirange projection](phase-2/postgres/181-phase-2-postgresql-multirange-projection.md)
- 182 — [Phase 2 PostgreSQL composite projection](phase-2/postgres/182-phase-2-postgresql-composite-projection.md)
- 183 — [Phase 2 PostgreSQL domain projection](phase-2/postgres/183-phase-2-postgresql-domain-projection.md)
- 184 — [Phase 2 PostgreSQL enum projection](phase-2/postgres/184-phase-2-postgresql-enum-projection.md)
- 185 — [Phase 2 PostgreSQL network projection](phase-2/postgres/185-phase-2-postgresql-network-projection.md)
- 186 — [Phase 2 PostgreSQL bit-string projection](phase-2/postgres/186-phase-2-postgresql-bit-string-projection.md)
- 187 — [Phase 2 PostgreSQL identifier projection](phase-2/postgres/187-phase-2-postgresql-identifier-projection.md)
- 188 — [Phase 2 PostgreSQL LSN projection](phase-2/postgres/188-phase-2-postgresql-lsn-projection.md)
- 189 — [Phase 2 PostgreSQL TID projection](phase-2/postgres/189-phase-2-postgresql-tid-projection.md)
- 190 — [Phase 2 PostgreSQL OID-vector projection](phase-2/postgres/190-phase-2-postgresql-oid-vector-projection.md)
- 191 — [Phase 2 PostgreSQL snapshot projection](phase-2/postgres/191-phase-2-postgresql-snapshot-projection.md)

## Phase 2 — Redis driver

- 90 — [Phase 2 Redis binary SCAN foundation evidence](phase-2/redis/90-phase-2-redis-binary-scan-foundation.md)
- 131 — [Phase 2 Redis service cancellation evidence](phase-2/redis/131-phase-2-redis-service-cancellation.md)
- 138 — [Phase 2 Redis pipeline partial-failure evidence](phase-2/redis/138-phase-2-redis-pipeline-partial-failure.md)
- 139 — [Phase 2 Redis TTL truth](phase-2/redis/139-phase-2-redis-ttl-truth.md)
- 141 — [Phase 2 Redis collection SCAN evidence](phase-2/redis/141-phase-2-redis-collection-scans.md)
- 142 — [Phase 2 Redis SCAN mutation-race evidence](phase-2/redis/142-phase-2-redis-scan-mutation-races.md)
- 143 — [Phase 2 Redis timeout and reconnect evidence](phase-2/redis/143-phase-2-redis-timeout-reconnect.md)
- 144 — [Phase 2 Redis TLS and authentication evidence](phase-2/redis/144-phase-2-redis-tls-authentication.md)
- 145 — [Phase 2 Redis Pub/Sub isolation evidence](phase-2/redis/145-phase-2-redis-pubsub-isolation.md)
- 146 — [Phase 2 Redis reviewed TTL mutation evidence](phase-2/redis/146-phase-2-redis-reviewed-ttl-mutation.md)
- 147 — [Phase 2 Redis pattern subscription evidence](phase-2/redis/147-phase-2-redis-pattern-subscriptions.md)
- 148 — [Phase 2 Redis Pub/Sub reconnect evidence](phase-2/redis/148-phase-2-redis-pubsub-reconnect.md)
- 149 — [Phase 2 Redis TLS Pub/Sub composition evidence](phase-2/redis/149-phase-2-redis-tls-pubsub.md)
- 150 — [Phase 2 Redis Pub/Sub ACL denial boundary](phase-2/redis/150-phase-2-redis-pubsub-acl-denial.md)
- 151 — [Phase 2 Redis live credential revocation evidence](phase-2/redis/151-phase-2-redis-live-credential-revocation.md)
- 152 — [Phase 2 Redis Pub/Sub credential revocation evidence](phase-2/redis/152-phase-2-redis-pubsub-credential-revocation.md)
- 153 — [Phase 2 Redis TLS Pub/Sub reconnect evidence](phase-2/redis/153-phase-2-redis-tls-pubsub-reconnect.md)
- 154 — [Phase 2 Redis TLS Pub/Sub replacement failure evidence](phase-2/redis/154-phase-2-redis-tls-pubsub-replacement-failure.md)
- 170 — [Phase 2 Redis atomic revocation fixture](phase-2/redis/170-phase-2-redis-atomic-revocation-fixture.md)
- 171 — [Phase 2 Redis subscription connect policy](phase-2/redis/171-phase-2-redis-subscription-connect-policy.md)
- 173 — [Phase 2 Redis administrative readiness budget](phase-2/redis/173-phase-2-redis-admin-readiness-budget.md)

## Phase 2 — ClickHouse driver

- 95 — [Phase 2 ClickHouse RowBinary foundation evidence](phase-2/clickhouse/95-phase-2-clickhouse-rowbinary-foundation.md)
- 97 — [Phase 2 ClickHouse complex scalar evidence](phase-2/clickhouse/97-phase-2-clickhouse-complex-scalars.md)
- 100 — [Phase 2 ClickHouse structured-container evidence](phase-2/clickhouse/100-phase-2-clickhouse-structured-containers.md)
- 129 — [Phase 2 ClickHouse service cancellation evidence](phase-2/clickhouse/129-phase-2-clickhouse-service-cancellation.md)
- 178 — [Phase 2 ClickHouse temporal projection](phase-2/clickhouse/178-phase-2-clickhouse-temporal-projection.md)

## Delivery and CI

- 192 — [CI verification baseline](delivery/192-ci-verification-baseline.md)

## TermRock migrations and updates

- 49 — [TermRock 0.8 canonical API migration evidence](termrock/49-termrock-0.8-migration.md)
- 57 — [TermRock 0.9 styled tab glyph migration evidence](termrock/57-termrock-0.9-migration.md)
- 59 — [TermRock 0.9 input and OSC migration evidence](termrock/59-termrock-0.9-input-osc-migration.md)
- 61 — [TermRock 0.9 unified key vocabulary migration evidence](termrock/61-termrock-0.9-key-vocabulary-migration.md)
- 64 — [TermRock 0.9 constructible theme migration evidence](termrock/64-termrock-0.9-constructible-theme-migration.md)
- 67 — [TermRock 0.9 semantic palette migration evidence](termrock/67-termrock-0.9-semantic-palette-migration.md)
- 70 — [TermRock 0.9 slate preset update evidence](termrock/70-termrock-0.9-slate-preset-update.md)
- 72 — [TermRock 0.9 neutral event migration evidence](termrock/72-termrock-0.9-neutral-event-migration.md)
- 74 — [TermRock 0.9 canonical module migration evidence](termrock/74-termrock-0.9-canonical-module-migration.md)
- 75 — [TermRock 0.10 metadata and selection migration evidence](termrock/75-termrock-0.10-metadata-selection-migration.md)
- 78 — [TermRock 0.10 widget construction migration evidence](termrock/78-termrock-0.10-widget-construction-migration.md)
- 80 — [TermRock 0.10 documentation hardening update](termrock/80-termrock-0.10-documentation-hardening-update.md)
- 82 — [TermRock 0.10 visible scroll update](termrock/82-termrock-0.10-visible-scroll-update.md)
- 83 — [TermRock 0.10 content revision migration](termrock/83-termrock-0.10-content-revision-migration.md)
- 85 — [TermRock 0.10 closure runner spike update](termrock/85-termrock-0.10-runner-spike-update.md)
- 86 — [TermRock 0.10 plan reconciliation update](termrock/86-termrock-0.10-plan-reconciliation-update.md)
- 88 — [TermRock 0.10 runtime keymap spike update](termrock/88-termrock-0.10-runtime-keymap-spike-update.md)
- 89 — [TermRock 0.10 lookbook output hardening update](termrock/89-termrock-0.10-lookbook-output-hardening-update.md)
- 91 — [TermRock 0.10 interactive story controls update](termrock/91-termrock-0.10-interactive-story-controls-update.md)
- 92 — [TermRock 0.10 picker spike update](termrock/92-termrock-0.10-picker-spike-update.md)
- 93 — [TermRock all-features and neutral input adoption](termrock/93-termrock-all-features-neutral-input-adoption.md)
- 94 — [TermRock 0.10 contract-axis story update](termrock/94-termrock-0.10-contract-axis-story-update.md)
- 96 — [TermRock 0.10 scroll/session migration evidence](termrock/96-termrock-0.10-scroll-session-migration.md)
- 102 — [TermRock 0.11 migration evidence](termrock/102-termrock-0.11-migration.md)
- 105 — [TermRock migration 0018 adoption](termrock/105-termrock-migration-0018.md)
- 108 — [TermRock migration 0019 adoption](termrock/108-termrock-migration-0019.md)
- 111 — [TermRock migration 0020 adoption](termrock/111-termrock-migration-0020.md)
- 113 — [TermRock migration 0021 adoption](termrock/113-termrock-migration-0021.md)
- 115 — [TermRock immutable frame-tick spike adoption](termrock/115-termrock-frame-tick-spike.md)
- 118 — [TermRock focus and table spike update](termrock/118-termrock-focus-table-spike-update.md)
- 121 — [TermRock textarea spike update](termrock/121-termrock-textarea-spike-update.md)
- 122 — [TermRock paste-payload migration](termrock/122-termrock-paste-payload-migration.md)
- 124 — [TermRock list multi-select migration](termrock/124-termrock-list-multiselect-migration.md)
- 128 — [TermRock semantic inventory update](termrock/128-termrock-semantic-inventory-update.md)
- 130 — [TermRock migration 0024 adoption](termrock/130-termrock-closure-runner-frame-time-migration.md)
- 132 — [TermRock migration 0025 adoption](termrock/132-termrock-runtime-keymap-migration.md)
- 134 — [TermRock Picker graduation update](termrock/134-termrock-picker-graduation-update.md)
- 137 — [TermRock migration 0026 adoption](termrock/137-termrock-scoped-focus-migration.md)
- 140 — [TermRock TextArea graduation and migration 0027](termrock/140-termrock-textarea-graduation-migration.md)
- 169 — [TermRock 0.11 lookbook event update](termrock/169-termrock-0.11-lookbook-event-update.md)
