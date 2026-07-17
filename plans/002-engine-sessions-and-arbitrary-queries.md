# Plan 002: Give the engine persistent sessions and an arbitrary-statement execution path

> **Executor instructions**: Follow this plan step by step. Run every
> verification command before moving on. On any STOP condition, stop and
> report. Update your row in `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat d8b113b..HEAD -- crates/tablerock-engine crates/tablerock-core`
> On any change to in-scope files, re-verify the "Current state" excerpts
> against live code; mismatch = STOP.

## Status

- **Priority**: P1
- **Effort**: L
- **Risk**: MED
- **Depends on**: plans/001-ci-verification-baseline.md
- **Category**: direction (structural unlock for every UI phase)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

The Phase 2 drivers are real (TLS, streaming, cancellation, rich decoders) but
their query surface is **spike-shaped**: PostgreSQL and ClickHouse accept only
hardcoded probe enums, and every session lives for exactly one operation — the
runtime shuts the session down when its single operation ends. No workbench can
be built on that: a UI needs one connection that survives many catalog loads
and queries, and it needs to run operator-supplied SQL. This plan converts the
spike surface into the product surface without discarding any proven behavior.
This is the single highest-leverage change in the repository; plans 003, 005,
006, 009 and everything after them depend on it.

## Current state

- `crates/tablerock-engine/src/adapter.rs:17-50` — `DriverPageRequest` enum:
  `PostgreSqlProbe { query: PostgresProbeQuery, … }`,
  `ClickHouseProbe { query: ClickHouseProbeQuery, … }`, plus four genuine Redis
  variants. The probe query types are fixed enums mapping to literal SQL:
  `PostgresProbeQuery` (`src/postgres.rs:124-146`, 21 variants),
  `ClickHouseProbeQuery` (`src/clickhouse.rs:37`, 5 variants). There is no way
  to pass operator SQL.
- `crates/tablerock-engine/src/adapter.rs:185-196` — `trait DriverSession`:
  `engine()`, `start_page_stream(request)`, `cancel(operation_id)`,
  `shutdown(self: Box<Self>)`. No health method, no session reuse contract.
- `crates/tablerock-engine/src/runtime.rs:245` (`run_operation`) — the spawned
  task **always** calls `session.shutdown()` at terminal (lines ~273/286/370/379):
  one session ⇒ one operation ⇒ disconnect.
- `crates/tablerock-engine/src/service.rs:90-134` — `EngineService::submit`
  takes `session: Box<dyn DriverSession>` by value and hands it to the
  runtime; the service never retains sessions.
- Concrete sessions: `PostgresSession::connect`/`connect_with_tls`
  (`src/postgres.rs:719/731`), `ClickHouseSession::connect`
  (`src/clickhouse.rs:216`, no network round-trip), `RedisSession::connect`
  (`src/redis.rs:1043`). ClickHouse and Redis sessions are `Clone`;
  `PostgresSession` is not.
- `crates/tablerock-core/src/command.rs:182-189` — `CommandIntent` has
  `TestProfile/Connect/Disconnect/RefreshCatalog/FetchPage/Cancel/Shutdown`
  but **no Execute intent** for running a statement.
- Error taxonomy: `AdapterError { engine, class }` with 14
  `AdapterFailureClass` variants (`adapter.rs:124-146`) — leak-free by design;
  raw driver errors never cross the adapter (decided; do not add a message
  field). Rich diagnostics travel separately via `SafeDiagnostic`
  (`tablerock-core/src/diagnostic.rs:143`).
- Evidence constraints that bind this plan (decided, not negotiable):
  bounded pages only (500-row default, byte budgets), cancellation reports
  observed outcomes only, ambiguous writes never retried, SQL text never in
  logs/Debug output. See `docs/architecture/fixed-decisions.md` ("Result
  budgets and encoding", "Safety") and `docs/architecture/shared-client-contract.md`.
- Repo conventions: constructor-validated owned types, `from_wire` versioning,
  exhaustive behavioral tests per module in `tests/`, no `unsafe`, workspace
  clippy denies. Exemplar for a validated request type:
  `tablerock-core/src/command.rs` `PageRequest::new` (:225).

## Commands you will need

| Purpose | Command | Expected |
|---|---|---|
| Build | `cargo check --workspace --all-targets` | exit 0 |
| Core tests | `cargo test -p tablerock-core` | pass |
| Engine unit | `cargo test -p tablerock-engine --lib` | pass |
| PG real (Docker) | `cargo test -p tablerock-engine --test postgres_real` | pass |
| CH real (Docker) | `cargo test -p tablerock-engine --test clickhouse_real` | pass |
| Redis real (Docker) | `cargo test -p tablerock-engine --test redis_real` | pass |
| Overlap (Docker) | `cargo test -p tablerock-engine --test three_engine_overlap_real` | pass |
| Lint | `cargo clippy --workspace --all-targets` | exit 0 |

## Scope

**In scope**:
- `crates/tablerock-core/src/command.rs` — add an `Execute` intent carrying a
  bounded statement (new `StatementText` type) scoped to `Context`.
- `crates/tablerock-core/src/value.rs` or a new core module — `StatementText`:
  bounded UTF-8 (pick 1 MiB cap), Debug-redacted (byte length only), following
  the `PasteText` redaction pattern (`tablerock-tui/src/message.rs:41-49`).
- `crates/tablerock-engine/src/adapter.rs` — new `DriverPageRequest` variants
  `PostgreSqlStatement { statement, limits, max_cell_bytes }` and
  `ClickHouseStatement { statement, query_id, limits, max_cell_bytes }`;
  extend `DriverSession` with `fn health<'a>(&'a self) -> DriverFuture<'a, Result<SessionHealth, AdapterError>>`.
- `crates/tablerock-engine/src/postgres.rs` — arbitrary-statement streaming
  reusing the existing `stream_probe` machinery (`simple_query`/extended path
  identical to probes but with caller SQL); `health` = trivial round-trip
  (`SELECT 1` via the existing prepared path).
- `crates/tablerock-engine/src/clickhouse.rs` — arbitrary statement via the
  proven `fetch_bytes("RowBinaryWithNamesAndTypes")` path; `health` = cheap
  `SELECT 1`.
- `crates/tablerock-engine/src/redis.rs` — `health` = existing PING readiness
  (see `redis_admin` readiness pattern in tests); no arbitrary-command surface
  in this plan (Phase 8 owns the command editor).
- `crates/tablerock-engine/src/runtime.rs` + `src/service.rs` — session
  retention: operations borrow a shared session instead of consuming it.
- `crates/tablerock-engine/src/session_pool.rs` (create) — `SessionRegistry`:
  owns `Arc<dyn DriverSession>` per `SessionId`, hands out clones for
  operations, explicit `disconnect(session_id)` calls shutdown exactly once.
- New/extended tests (see Test plan).
- Evidence docs + `docs/architecture/` update for the session-ownership change
  + `ROADMAP.md` Phase 2 "still open" line adjustments + parity-ledger review
  rows, per repo rule.
- `.github/workflows/checks.yml` — add any new `--test` targets.

**Out of scope** (do NOT touch):
- TUI/CLI crates (plan 005 wires the UI).
- Catalog listing (plan 003).
- Secret resolution / profiles (plan 004).
- Mutation execution paths (plan 013+ own writes; `CommandIntent` gets no
  mutation variant here).
- Any weakening of `AdapterError`'s leak-free shape (no message strings).
- The Redis pub/sub, blocking-pop, TTL-mutation surfaces — already correct.

## Git workflow

Trunk-only per `AGENTS.md`: direct commits to `main`, Conventional Commits,
`git commit -s`, push each checkpoint immediately. Split this plan into at
least four buildable checkpoint commits matching Steps 1/2/3/5 (each with its
evidence doc). Never leave `main` red.

## Steps

### Step 1: Core `Execute` intent + `StatementText`

Add `StatementText` (bounded, validated UTF-8, Debug shows byte length only —
NEVER the SQL; repo rule: SQL text absent from logs by default). Add
`CommandIntent::Execute { statement: StatementText }` with
`CommandSafety` — introduce a new `CommandSafety::MayWrite` variant and map
`Execute` to it (unknown statements are treated as writes per
`CONTRIBUTING.md` Safety section). Extend `scope_matches`
(`command.rs:209-220`): `Execute` is valid for `CommandScope::Context(_)`.
Update every exhaustive match on `CommandIntent`/`CommandSafety` the compiler
flags.

**Verify**: `cargo test -p tablerock-core` → pass, including new tests
(statement bounds, Debug redaction, scope matching).

### Step 2: Session registry + runtime borrow semantics

Create `SessionRegistry` in `crates/tablerock-engine/src/session_pool.rs`:
- `register(session_id: SessionId, session: Box<dyn DriverSession>) -> Result<(), _>`
  storing `Arc<dyn DriverSession>`;
- `session(session_id) -> Option<Arc<dyn DriverSession>>`;
- `async disconnect(session_id) -> Result<(), AdapterError>` — removes and
  calls `shutdown` exactly once (`Arc::try_unwrap` after operation drain, or an
  internal `Mutex<Option<Box<dyn DriverSession>>>` design — pick one and state
  it in the evidence doc);
- bounded capacity (reuse the `ServiceLimits` scale: cap 1024).

Change `DriverRuntime::spawn` to accept `Arc<dyn DriverSession>` and REMOVE
the unconditional `session.shutdown()` at operation end (`runtime.rs`
~:273/286/370/379) — dropping the `Arc` clone replaces it. `DriverSession`
methods already take `&self`, so `Arc<dyn DriverSession>` works without
signature changes; `shutdown(self: Box<Self>)` stays reachable only through
the registry. Preserve the existing cancel/client-stop/terminal sequencing
exactly — those orderings are evidence-backed (`docs/evidence/phase-2/core/123…`).
Update `EngineService::submit` to take `Arc<dyn DriverSession>`; add
`EngineService::disconnect(session_id)` delegating to the registry after
verifying no active operation holds the session (else return a new
`EngineServiceError::SessionBusy`).

**Verify**: `cargo test -p tablerock-engine --lib` and
`cargo test -p tablerock-engine --test engine_service` → pass (update fake
sessions in `tests/support/mod.rs` to the `Arc` signature).

### Step 3: PostgreSQL + ClickHouse arbitrary statements, health

Add `DriverPageRequest::{PostgreSqlStatement, ClickHouseStatement}` and route
them in the respective `DriverSession::start_page_stream` impl blocks
(`adapter.rs:282+/324+`) to new session methods:
- `PostgresSession::stream_statement(statement, limits, max_cell_bytes)` —
  same streaming/decoding/truncation path as `stream_probe` (`postgres.rs:788`)
  but with caller SQL; reuse `decode_columns`/`decode_value` unchanged.
- `ClickHouseSession::stream_statement(statement, query_id, limits, max_cell_bytes)`
  mirroring `stream_probe` (`clickhouse.rs:241`), keeping the single-active-query
  guard (`SessionBusy`).
Add `health()` to the trait + all three impls (PG `SELECT 1`, CH `SELECT 1`,
Redis PING). Return a small owned `SessionHealth { engine, server_reachable: bool, elapsed_millis: u64 }`
(no version strings in this plan — Test Connection facts belong to plan 004/006).
Keep the probe enums and every existing probe path intact — real-server
evidence depends on them.

**Verify (Docker)**: `cargo test -p tablerock-engine --test postgres_real --test clickhouse_real` → pass, including new tests below.

### Step 4: Multi-operation session proof

Extend real-server tests: one `PostgresSession` (via registry) runs, in
sequence, a statement returning typed rows, then a second statement, then
cancellation of a long statement (`pg_sleep` pattern from
`PostgresProbeQuery::CancellationStream`, now expressed as caller SQL), then
`health`, and the session stays usable throughout; same shape for ClickHouse.
Hostile-input cases: syntax error surfaces as `AdapterFailureClass::Query`
with the session still usable; oversized statement rejected pre-I/O.

**Verify (Docker)**: the four real-server suites + overlap suite all pass.

### Step 5: Docs, evidence, roadmap

Architecture: update `docs/architecture/shared-client-contract.md` (session
ownership paragraph) and `docs/architecture/rust-core-architecture.md` if it
names single-operation sessions. Evidence: one doc per checkpoint commit
(core intent; registry/runtime; PG statement; CH statement; health), each with
bounds + failure truth + verification, plus index lines. Parity ledger: update
the "Query/command tabs" row's evidence pointer. ROADMAP Phase 2 "Still open"
paragraph: remove items this plan closes, keep the rest.

**Verify**: `cargo check --workspace --all-targets && cargo clippy --workspace --all-targets` → exit 0; evidence index renders the new rows.

## Test plan

- `tablerock-core/tests/command.rs`: `Execute` scope/safety/redaction tests +
  `StatementText` bounds & Debug redaction (model after existing
  `PageRequest`/`PasteText` tests).
- `tablerock-engine/tests/engine_service.rs`: submit-with-Arc, disconnect-once,
  disconnect-while-busy rejection, spawn-failure still transitions Failed.
- New `tablerock-engine/tests/session_registry.rs`: capacity, double-register,
  double-disconnect, shutdown-exactly-once (use a counting fake session).
- Real-server additions per Step 4 in `postgres_real.rs` / `clickhouse_real.rs`.
- Verification: all commands in the table green; CI (`checks.yml`) green after
  push.

## Done criteria

- [ ] `CommandIntent::Execute` exists; `grep -n "Execute" crates/tablerock-core/src/command.rs` shows intent + scope match + safety mapping
- [ ] `grep -rn "session.shutdown" crates/tablerock-engine/src/runtime.rs` returns no unconditional terminal-path shutdown
- [ ] A real-server test proves ≥3 sequential operations + cancel + health on ONE PostgreSQL session and ONE ClickHouse session
- [ ] Probe enums and all pre-existing tests still pass unchanged
- [ ] `cargo clippy --workspace --all-targets` exit 0
- [ ] Evidence docs added + indexed; ledger/roadmap rows updated
- [ ] `plans/README.md` row updated

## STOP conditions

- Converting the runtime to `Arc<dyn DriverSession>` requires changing
  `DriverSession` method receivers (they are `&self` today at
  `adapter.rs:185-195`) — if live code differs, STOP.
- Any existing real-server test changes observable cancellation/ambiguity
  semantics to pass — those are evidence-locked; STOP and report.
- ClickHouse arbitrary statements cannot reuse `RowBinaryWithNamesAndTypes`
  for some statement class (e.g. DDL returning no shape) — implement the
  result-less command path only if trivially expressible as a zero-row final
  page; otherwise STOP and report the shape.
- You need to add a dependency — dependency adoption requires its own
  checkpoint approval per `AGENTS.md`; STOP.

## Maintenance notes

- Plan 003 adds catalog request variants beside `Execute`; plan 005 consumes
  `EngineService` + registry from the TUI; plan 019 exposes the same seam over
  UniFFI. Keep the registry API coarse (open/submit/events/page/cancel/
  shutdown shape per `docs/architecture/shared-client-contract.md:60-66`).
- Reviewer scrutiny: session teardown races (operation still streaming while
  disconnect called), and that no code path formats `StatementText` into an
  error, log, or Debug string.
- Deferred: statement-level safety classification (parser-based read/write
  detection) — Phase 5/6 concern with `sqlparser`; `Execute` stays `MayWrite`
  until then.
