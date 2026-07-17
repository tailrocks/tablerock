# Plan 006: Deliver the Phase 3 connection experience — list, groups, editor, Test/Connect

> **Executor instructions**: Follow step by step; verify each step; STOP
> conditions binding. Update `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat d8b113b..HEAD -- crates/tablerock-tui crates/tablerock-cli crates/tablerock-engine/src/service.rs`
> Compare "Current state" excerpts on any change; mismatch = STOP. Requires
> plans 002, 004, 005 DONE (check `plans/README.md`).

## Status

- **Priority**: P1
- **Effort**: L
- **Risk**: MED
- **Depends on**: plans/002, plans/004, plans/005
- **Category**: direction (Phase 3 roadmap exit)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

This is the first user-visible product capability: everything before it is
substrate. Exit gate is ROADMAP Phase 3 / `docs/architecture/delivery-plan.md`
"Phase 3 — profiles and connection experience". The full behavioral spec is
`docs/product/connections.md` — the executor MUST read that file first; it is
the authority for every state on these screens.

## Current state

- Screens today are placeholders: `Screen::{Connections, ConnectionPicker}`
  render titled empty panels (`crates/tablerock-tui/src/view.rs:136-181`).
  After plan 005 the Connections screen shows a profile count via the effect
  executor; submodels/effects/projections exist.
- Domain ready (plan 004): `ProfileAggregate` with `EnvironmentTag`, group
  ops on `PersistenceActor`, host/database search, `resolve_for_connect`
  (prompt + dangerous plaintext, others fail closed).
- Engine ready (plan 002): persistent sessions via `SessionRegistry`,
  `DriverSession::health`, per-engine connect constructors:
  `PostgresSession::connect/connect_with_tls` (`postgres.rs:719/731`),
  `ClickHouseSession::connect` (`clickhouse.rs:216` — no network round-trip;
  first error surfaces at query time), `RedisSession::connect`
  (`redis.rs:1043`).
- TermRock widgets available at pinned rev `b7f34da` (workspace
  `Cargo.toml:24`): `Tree`, `Form`, `List`, `TextInput`, `Dialog`,
  `SplitPane`, `Toast`, `DetailTable`, plus the already-used
  Panel/Tabs/ActionBar/StatusBar/HintBar. Do NOT build local substitutes —
  Phase 3 exit evidence requires "profile forms use TermRock `Form`/`Tree`
  rather than local substitutes" (`delivery-plan.md` Phase 3 exit).
- Command intents exist: `TestProfile`, `Connect`, `Disconnect`
  (`tablerock-core/src/command.rs:182-189`) scoped Profile/Profile/Session.
- Spec anchors the executor must honor (`docs/product/connections.md`):
  - List rows: name, engine badge, `host:port/database`, environment tag
    (label + color, never color alone), safety mode, plaintext-secret
    warning, live state (disconnected/connecting/connected/reconnecting/
    failed). Explicit empty/loading/failed screens.
  - Groups: collapsible, profile in ≤1 group, create/rename/delete
    (delete moves members to ungrouped), manual or alphabetical ordering.
  - Editor first version has EXACTLY: engine, name, group, environment tag,
    host, port, default database (Redis: logical DB index), username,
    password, TLS mode (off / verify CA / verify full). Nothing else.
  - Password: ONE field, one source; sources now = prompt-on-connect,
    save-locally-dangerous (acknowledged, persistent warning).
  - Test: server identity/version, TLS outcome, elapsed, redacted failure —
    without saving. Connect: saves (or temporary) and opens workbench.
  - Temporary connection: nothing durable after quit.
  - Failure truth: unresolved secret fails before network I/O naming the
    source, never the value; bounded backoff reconnect stops on auth failure;
    removing a profile with history/active sessions asks first.

## Commands you will need

| Purpose | Command | Expected |
|---|---|---|
| TUI/CLI tests | `cargo test -p tablerock-tui -p tablerock-cli` | pass |
| Engine/persistence | `cargo test -p tablerock-engine --lib -p tablerock-persistence` | pass |
| Real-server (Docker) | `cargo test -p tablerock-engine --test postgres_real --test clickhouse_real --test redis_real` | pass |
| Build/lint | `cargo check --workspace --all-targets && cargo clippy --workspace --all-targets` | exit 0 |

## Scope

**In scope**:
- `crates/tablerock-tui/src/model/connections.rs` (grow from plan 005's
  profiles submodel): list state (grouped rows, collapse state, selection,
  search text), editor state (`ConnectionFormModel` — deliberately
  TableRock-local per `docs/architecture/termrock-integration.md`
  "Deliberately TableRock-local" table), test-result state, dialogs
  (remove-confirm, plaintext acknowledgement, password prompt).
- `crates/tablerock-tui/src/view/connections.rs` — list via TermRock
  `Tree` (groups) + `List` + `TextInput` (search); editor via `Form` with
  General/Connection/Credentials/TLS sections; dialogs via `Dialog`;
  warnings text+glyph.
- Messages/effects: CRUD effects (create/replace/delete/duplicate via
  `PersistenceActor`), group ops, `TestProfile` and `Connect` effects (build
  engine session from snapshot + resolved secret; Test = connect→health→
  identity facts→shutdown WITHOUT registering; Connect = register in
  `SessionRegistry` + mark session live), disconnect.
- `crates/tablerock-cli/src/effects.rs` + `projection.rs` — extend.
- Engine: a `TestOutcome { server_identity, server_version, tls_outcome, elapsed_millis }`
  fact — add a `describe_server` capability to sessions (PG: `SELECT version()`;
  CH: `SELECT version()`; Redis: INFO server subset) with bounded redacted
  output. Small extension, lives beside plan 002's `health`.
- Reconnect: bounded-backoff loop in the executor with auth-stop (map
  `AdapterFailureClass::Authentication` to stop).
- Tests + evidence docs + parity-ledger rows (Connection list,
  Create/edit/duplicate/remove, Engine chooser, Test connection, Temporary
  connection, Environment tag, Health and reconnect) + ROADMAP Phase 3 status.

**Out of scope** (spec-deferred; render nothing for them):
- URL import, external URL open, 1Password/Keychain/env sources, SSH, custom
  CA editing UI, startup actions, favorites-beyond-groups.
- The workbench itself (plan 007) — Connect may land on a stub workbench
  screen showing session facts.

## Git workflow

Trunk-only, Conventional Commits, `git commit -s`, push per checkpoint.
Suggested checkpoints: list+groups read-only → editor+CRUD → Test/Connect →
reconnect+policies. Each with evidence.

## Steps

### Step 1: Grouped list + search (read-only)

Submodel + view: grouped rows from `list_profiles` (+ `list_groups`),
collapse/expand, keyboard navigation, search box filtering by name/host/
database/group preserving group structure, explicit empty/loading/failed
states (spec: never blank areas). Live-state column renders `disconnected`
for all rows until Step 3. Render tests for all states incl. narrow layout.

**Verify**: `cargo test -p tablerock-tui` → pass.

### Step 2: Editor + CRUD + groups

`ConnectionFormModel` with exactly the spec fields; per-engine field labels
(database vs logical DB index); TLS mode select mapping to `TlsPolicy`
(`Disabled`/`VerifySystemRoots`/`VerifyCustomCa` is NOT offered yet — spec
says "custom CA later"; offer off/verify-CA/verify-full mapping to
`Disabled`/`VerifySystemRoots` + server-name verify per engine config;
if `TlsPolicy` cannot express verify-CA vs verify-full distinctly, STOP).
Password source selector (prompt / save-locally-dangerous with typed
acknowledgement dialog). Save via revision-CAS `replace_profile`; duplicate =
load→new ID→create; remove = confirm dialog. Group create/rename/delete
dialogs calling plan 004 actor ops. Reducer tests: validation failures,
CAS-conflict surfaced (stale revision → reload prompt), acknowledgement
required before plaintext save.

**Verify**: `cargo test -p tablerock-tui -p tablerock-cli` → pass.

### Step 3: Test / Connect / Temporary / Disconnect

Effects: build engine config from `ProfileConnectionSnapshot` +
`resolve_for_connect` (password prompt modal wired as the `SecretPromptPort`
through a dedicated effect/message round trip — the port cannot block the
reducer; design it as: reducer opens prompt modal → user submits →
effect resumes with the secret, secret lives only inside the effect executor).
Test: connect → `describe_server` + `health` → shutdown; render identity/
version/TLS outcome/elapsed; failure renders redacted class. Connect:
register session, transition list row live-state, open stub workbench screen.
Temporary: same as Connect with `ProfileDurability::Temporary` (never
persisted — verify by DB inspection in test). Disconnect: registry
disconnect + row state update. Real-server integration test (Docker):
Test+Connect+Disconnect against all three engines incl. wrong-password
(redacted auth failure) and unreachable-host (redacted connect failure).

**Verify (Docker)**: engine real-server suites + `cargo test -p tablerock-cli` → pass.

### Step 4: Reconnect, policies, removal safety

Bounded-backoff reconnect in executor (e.g. 1s/2s/4s/8s cap 30s, stop on
`Authentication`), state renders `reconnecting`; removal of a profile with an
active session asks (dialog) — history linkage is deferred until history
exists (note in evidence). Quit with temporary session leaves no durable
row (test: relaunch actor, assert absent).

**Verify**: `cargo test -p tablerock-tui -p tablerock-cli` → pass.

### Step 5: Evidence + ledger + roadmap

Phase 3 exit evidence per `delivery-plan.md`: all engines pass local + TLS
fixtures; picker/search never resolves secrets (test: secret resolution
counter untouched by list/search); Test/Connect resolves only requested
fields; temporary leaves nothing durable; reconnect never repeats an
ambiguous write (reconnect never resubmits operations — assert executor has
no retry path); forms use TermRock Form/Tree. Update ledger rows + ROADMAP
Phase 3 → complete with evidence links.

**Verify**: full command table green; CI green.

## Test plan

- Reducer: state machines for list/editor/dialogs/prompt; CAS conflict;
  acknowledgement gate; search preservation of groups.
- Render (`TestBackend`, pattern `tests/shell.rs`): every spec state incl.
  empty/loading/failed, production-tag warning treatment, plaintext warning,
  narrow layout.
- Integration: real-server Test/Connect matrix; temporary-profile
  durability; secret-resolution-count assertions.
- Exemplars: `tests/shell.rs` (render), plan 005's vertical test (end-to-end
  shape), `tests/redis_real.rs` TLS/ACL fixtures (engine-side patterns).

## Done criteria

- [ ] Every `docs/product/connections.md` "Connection list" fact renders (row facts, states, actions)
- [ ] Editor has exactly the spec fields; nothing more (review view code)
- [ ] Test works without saving on all three engines (Docker test)
- [ ] Temporary connection leaves no durable profile/secret (test)
- [ ] Unresolved/failing secret fails before network I/O naming source only (test)
- [ ] Reconnect backoff stops on auth failure (test with rotated password fixture)
- [ ] TermRock `Form`/`Tree` used — `grep -rn "struct.*Form\|struct.*Tree" crates/tablerock-tui/src/` shows no local generic form/tree widget
- [ ] Evidence docs + ledger + ROADMAP updated; clippy green; `plans/README.md` updated

## STOP conditions

- `TlsPolicy` can't express the spec's verify-CA vs verify-full distinction
  per engine — STOP (core contract decision).
- TermRock `Form`/`Tree`/`List` lack a needed neutral capability (e.g.
  collapsible sections with stable IDs) — STOP: the fix is a TermRock-first
  contribution per `docs/architecture/termrock-integration.md` "TermRock
  contribution gate" (9 requirements incl. lookbook + Jackin compatibility);
  that is its own checkpoint, possibly its own plan.
- The password-prompt port cannot be expressed without blocking the reducer
  or holding secret bytes in the Model — STOP (Model must never hold a
  resolved secret; `application-pattern.md` Model rules).
- ClickHouse "Test" cannot distinguish reachability (its connect is lazy) —
  acceptable: Test's health query does the round trip; but if identity facts
  can't be fetched, STOP rather than fake them.

## Maintenance notes

- Plan 007 replaces the stub workbench with the real shell; keep the
  post-Connect handoff to a single message (`Message::Workbench(Opened{…})`).
- Reviewer: no secret bytes in Model/messages/Debug; CAS conflicts always
  surfaced; every list state reachable by keyboard only.
- Deferred rows stay visible in the parity ledger (URL import, secret
  sources, SSH) — do not mark Phase 3 rows closed that this plan didn't ship.
