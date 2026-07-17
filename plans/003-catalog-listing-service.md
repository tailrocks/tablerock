# Plan 003: Implement catalog listing for all three engines behind RefreshCatalog

> **Executor instructions**: Follow step by step; verify each step; STOP
> conditions are binding. Update `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat d8b113b..HEAD -- crates/tablerock-engine crates/tablerock-core/src/catalog.rs`
> Compare "Current state" excerpts on any change; mismatch = STOP. Note this
> plan assumes plan 002 landed (sessions are `Arc`-shared and reusable).

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: plans/002-engine-sessions-and-arbitrary-queries.md
- **Category**: direction
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

The workbench sidebar (product spec `docs/product/workbench.md` "Sidebar
catalog") needs databases/schemas/tables/views/functions (PostgreSQL),
databases/tables/views/dictionaries (ClickHouse), and logical databases/key
namespaces (Redis). `tablerock-core` already ships a complete validated
catalog contract — `CatalogSnapshot`, `CatalogNode`, lazy `CatalogChildrenState`,
`CatalogCursor` (`crates/tablerock-core/src/catalog.rs`) — and
`CommandIntent::RefreshCatalog` exists (`command.rs:185`), but **no engine
code produces a snapshot**: there is no catalog method on `DriverSession` and
no `DriverPageRequest` variant for metadata. Phase 4/7/8 screens are blocked
on this.

## Current state

- Core contract (consume, do not redesign):
  - `CatalogNodeKind` — 10 variants across engines (`catalog.rs:40`).
  - `CatalogChildrenState::{NotApplicable, Unrequested, Loading, Loaded{complete}, Stale, Failed}` (`catalog.rs:95`).
  - `CatalogSnapshot::new` validates a preorder tree against `CatalogLimits`
    (`catalog.rs:302`); `CatalogCursor::accept` takes next-revision snapshots
    only (`catalog.rs:453`).
- Engine gap: `DriverPageRequest` (`adapter.rs:17-50`, plus plan 002's
  statement variants) has no catalog variant; `DriverSession`
  (`adapter.rs:185`) has no catalog method.
- PostgreSQL session (`postgres.rs`) has full prepared-statement machinery and
  typed decoders — catalog queries are ordinary bounded queries against
  `pg_catalog` (`pg_namespace`, `pg_class`, `pg_proc`, `pg_database`).
- ClickHouse: `system.databases`, `system.tables`, `system.dictionaries` via
  the proven RowBinary path.
- Redis: `scan_keys` exists (`redis.rs:1491`, SCAN-based, never `KEYS`);
  logical databases enumerate 0..databases from `CONFIG GET databases`
  (fallback 16 when CONFIG is denied — record as a capability fact, not an
  error).
- Product requirements that bind the shape (`docs/product/workbench.md`):
  lazy expansion with explicit loading/stale/error per node; subtree refresh;
  functions listed with argument signature; unsupported kinds hidden or
  explicit, never empty sections. Redis namespaces are projections split on
  `:` and are NOT produced engine-side — the engine returns keys; namespace
  grouping is a UI model concern (`docs/product/redis.md` "Sidebar").
- Fixed decision: engine differences are capabilities, not fake normalization
  (`docs/architecture/delivery-plan.md` Phase 2 exit).

## Commands you will need

| Purpose | Command | Expected |
|---|---|---|
| Build | `cargo check --workspace --all-targets` | exit 0 |
| Core tests | `cargo test -p tablerock-core` | pass |
| Engine unit | `cargo test -p tablerock-engine --lib` | pass |
| Real servers (Docker) | `cargo test -p tablerock-engine --test postgres_real --test clickhouse_real --test redis_real` | pass |
| Lint | `cargo clippy --workspace --all-targets` | exit 0 |

## Scope

**In scope**:
- `crates/tablerock-engine/src/adapter.rs` — new request enum
  `CatalogRequest` (owned, bounded): per-engine variants such as
  `PostgreSqlDatabases`, `PostgreSqlSchemas { database }`,
  `PostgreSqlRelations { database, schema }` (tables+views+functions with
  signatures), `ClickHouseDatabases`, `ClickHouseObjects { database }`,
  `RedisLogicalDatabases`; new `DriverSession` method
  `fn catalog<'a>(&'a self, request: CatalogRequest) -> DriverFuture<'a, Result<CatalogSubtree, AdapterError>>`
  where `CatalogSubtree` is an owned bounded node list ready for
  `CatalogSnapshot` assembly.
- `crates/tablerock-engine/src/postgres.rs`, `src/clickhouse.rs`,
  `src/redis.rs` — implementations with bounded row/byte limits (reuse
  `PageLimits` scale) and identifier-safe parameterized queries.
- `crates/tablerock-engine/src/service.rs` — route
  `CommandIntent::RefreshCatalog` submissions: new
  `EngineService::refresh_catalog(session_id, scope, request)` producing a
  validated `CatalogSnapshot` at the next scope revision (advance the scope
  through `ServiceCoordinator::advance_scope`).
- Tests + evidence docs + parity-ledger "Lazy catalog" row update.
- `.github/workflows/checks.yml` — include any new test targets.

**Out of scope**:
- UI tree rendering (plan 009), Redis namespace projection model (plan 015),
  structure/DDL detail views (plans 009/013/014), permissions-aware filtering
  beyond what the catalogs naturally return.
- Any change to `tablerock-core/src/catalog.rs` semantics — if the contract
  cannot express something, STOP instead.

## Git workflow

Trunk-only, Conventional Commits, `git commit -s`, push per buildable
checkpoint (suggest: request/trait commit; one commit per engine; service
routing commit).

## Steps

### Step 1: `CatalogRequest` + trait method + `CatalogSubtree`

Owned bounded types in `adapter.rs` (or a new `src/catalog_request.rs`
module re-exported from `lib.rs`). `CatalogSubtree` carries: parent path
(engine-typed), `Vec<CatalogNodeSeed>` (kind, name as `BoundedText`,
children-state hint, optional signature text for functions), and a
completeness flag. Debug output must not print object names (identifier
redaction rule — match `CatalogSnapshot`'s name-redaction behavior, see core
`tests/catalog.rs` redaction tests).

**Verify**: `cargo check -p tablerock-engine` → exit 0.

### Step 2: PostgreSQL catalog queries

`pg_catalog`-based, ordered, bounded:
- databases: `SELECT datname FROM pg_database WHERE datallowconn ORDER BY datname LIMIT $1`
- schemas: `pg_namespace` excluding `pg_toast`/`pg_temp_%` prefixes,
- relations: `pg_class` relkind in `('r','p','v','m')` + `pg_proc` with
  `pg_get_function_arguments(oid)` for the signature.
All parameters bound (never formatted into SQL); limits enforced; overflow →
`Loaded { complete: false }` hint. Real-server test: create fixture schema
with tables/views/functions incl. a Unicode name and a name containing `"; --`,
assert exact listing + no injection + bounded truncation.

**Verify (Docker)**: `cargo test -p tablerock-engine --test postgres_real` → pass.

### Step 3: ClickHouse catalog queries

`system.databases` / `system.tables` (`engine`, `name`) /
`system.dictionaries`, same bounding rules, via RowBinary path. Real-server
test mirrors Step 2 (fixture db + table + view + dictionary).

**Verify (Docker)**: `cargo test -p tablerock-engine --test clickhouse_real` → pass.

### Step 4: Redis logical databases

`CONFIG GET databases` on the control connection → seeds `db0..dbN` nodes;
on ACL denial of CONFIG return the documented default 16 with a
`CapabilityFact`-style completeness marker (extend `CatalogSubtree` with an
`exactness` field rather than guessing). Real-server test incl. ACL-denied
CONFIG case (reuse the ACL fixture machinery in `tests/redis_real.rs`).

**Verify (Docker)**: `cargo test -p tablerock-engine --test redis_real` → pass.

### Step 5: Service routing + snapshot assembly

`EngineService::refresh_catalog` submits a `RefreshCatalog` command envelope
through the coordinator (Context scope), calls `session.catalog(request)`,
assembles `CatalogSnapshot::new` at `scope_revision.checked_next()`, advances
the scope, and returns the snapshot. Failure → snapshot untouched, scope
revision unchanged, error surfaced as `SafeDiagnostic`-compatible
`AdapterError`. Unit-test with a fake session (extend
`tests/support/mod.rs`); prove stale-cursor rejection using `CatalogCursor`.

**Verify**: `cargo test -p tablerock-engine --lib --test engine_service` → pass.

### Step 6: Docs/evidence/ledger

Evidence doc per engine + one for service routing; parity ledger "Lazy
catalog" row updated (driver subtree refresh + function listing now proven;
UI projection still open); roadmap Phase 2/4 notes adjusted.

**Verify**: full command table green.

## Test plan

- Fake-session unit tests: routing, revision advancement, failure atomicity.
- Real-server: exact listings, hostile identifiers, Unicode, bounds/overflow,
  ACL-denied CONFIG (Redis), empty database/schema cases (explicit empty,
  not error).
- Model tests after existing patterns in `postgres_real.rs`
  (`streams_typed_values_…` fixture style).

## Done criteria

- [ ] `DriverSession::catalog` exists and all three engines implement it
- [ ] Real-server tests prove PG tables+views+functions-with-signatures, CH tables+views+dictionaries, Redis logical DBs incl. denial fallback
- [ ] Injection test: object named `"; --` listed verbatim, never executed
- [ ] `EngineService::refresh_catalog` returns validated `CatalogSnapshot`; stale cursor rejected
- [ ] clippy + all suites green; evidence + ledger updated
- [ ] `plans/README.md` row updated

## STOP conditions

- The core `CatalogSnapshot`/`CatalogLimits` contract cannot express a needed
  fact (e.g. function signatures exceed text limits) — STOP; core changes
  need their own decision.
- Plan 002's session registry is absent in live code — STOP (dependency not
  met).
- Redis CONFIG fallback would require guessing beyond the documented
  16-database default — STOP.

## Maintenance notes

- Plan 009 consumes snapshots for the sidebar; plan 015 builds namespace
  projection on top of `scan_keys`, not on this catalog path.
- Reviewer: check every catalog query is parameterized and every listing is
  ordered + bounded; check Debug redaction of `CatalogSubtree`.
- Deferred: structure/DDL detail (columns, indexes, engine facts) — plans
  009/013/014; permission-aware catalogs (`has_schema_privilege`) — Phase 10.
