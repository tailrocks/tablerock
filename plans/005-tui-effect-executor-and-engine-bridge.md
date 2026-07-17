# Plan 005: Build the TUI effect executor, engine/persistence bridge, and screen-submodel architecture

> **Executor instructions**: Follow step by step; verify each step; STOP
> conditions binding. Update `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat d8b113b..HEAD -- crates/tablerock-tui crates/tablerock-cli`
> Compare "Current state" excerpts on any change; mismatch = STOP. Requires
> plan 002 (EngineService with session registry) and plan 004
> (PersistenceActor group/tag API) to be DONE; check `plans/README.md`.

## Status

- **Priority**: P1
- **Effort**: L
- **Risk**: MED
- **Depends on**: plans/002, plans/004
- **Category**: direction
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

The Phase 1 shell is a pure TEA loop with exactly one effect (`Exit`), no
async effect executor, and a dead engine-event channel (the sender is dropped
at startup). Every product screen needs the same three mechanisms this plan
builds once: (1) an effect executor that runs engine/persistence work on Tokio
and feeds typed completion messages back; (2) an in-process engine port
(`EngineService` + `PersistenceActor` behind one adapter) per
`docs/architecture/shared-client-contract.md` "TUI adapter"; (3) a screen/
submodel architecture so `Model` can grow past chrome state without breaking
TEA. Plans 006–018 all build screens on top of this.

## Current state

- `crates/tablerock-tui/src/effect.rs:3-6`:

  ```rust
  pub enum Effect {
      Exit,
  }
  ```

- `crates/tablerock-tui/src/model.rs:84-96` — `Model` holds only chrome:
  theme, keymap, width/height, `FocusRing`, `action: ActionId`,
  `screen: Screen` (2 variants: `Connections`, `ConnectionPicker`,
  `model.rs:45-48`), pointer/hover state, `engine_resync_required`.
- `crates/tablerock-tui/src/message.rs:52-77` — `Message` is terminal facts +
  focus/action intents + `EngineResyncRequired/EngineResynchronized`; no
  domain payloads.
- `crates/tablerock-cli/src/run.rs` — current-thread Tokio runtime,
  `tokio::select!` loop over crossterm `EventStream`, ingress channel, signals
  (`run.rs:196-208`); engine ingress sender dropped immediately (`run.rs:123`);
  comment at `run.rs:22` marks the missing producer. `Effect::Exit` handled
  inline (`run.rs:240-242`).
- Architecture guard (`crates/tablerock-tui/tests/architecture.rs`):
  `update.rs`/`view.rs` must contain no `.await`/`tokio::`/IO tokens, and the
  TUI manifest may depend ONLY on `ratatui-core` + `termrock`. **Therefore the
  engine bridge and all domain types flow through `tablerock-cli` (adapter
  crate), and the TUI expresses domain state via its own owned projection
  types** — OR the manifest rule is consciously revised. Decision for this
  plan: keep `tablerock-tui` pure; define presentation-projection types inside
  `tablerock-tui` (plain data, no deps) and let `tablerock-cli` translate
  core/engine types into them. Revising the manifest guard instead requires
  an architecture-doc change — STOP condition if you believe it necessary.
- TEA rules that bind this plan (`docs/architecture/application-pattern.md`):
  one root Message enum with nested feature enums; update sync/deterministic;
  effects carry operation identity/budget/redaction; bounded queues; stale
  events rejected by revision in the root reducer.
- Ingress loss policy: overflow → `ResyncRequired` (`ingress.rs:74-88`) —
  keep it.

## Commands you will need

| Purpose | Command | Expected |
|---|---|---|
| TUI tests | `cargo test -p tablerock-tui` | pass |
| CLI tests | `cargo test -p tablerock-cli` | pass (PTY tests need a real TTY-capable env) |
| Build/lint | `cargo check --workspace --all-targets && cargo clippy --workspace --all-targets` | exit 0 |

## Scope

**In scope**:
- `crates/tablerock-tui/src/effect.rs` — real effect vocabulary (see Step 1).
- `crates/tablerock-tui/src/message.rs` — nested feature messages
  (`Message::Profiles(ProfilesMsg)`, `Message::Engine(EngineMsg)`, …).
- `crates/tablerock-tui/src/model.rs` — submodel organization
  (`model/` directory split per `application-pattern.md:124-140` layout:
  `model.rs`, feature submodels under `src/model/`), screen enum growth.
- `crates/tablerock-tui/src/update.rs` — root reducer delegating to pure
  feature reducers; revision/stale-event checks at the root.
- New `crates/tablerock-cli/src/effects.rs` — the executor: owns
  `EngineService`, `SessionRegistry`, `PersistenceActor`, and a bounded
  request→completion pipeline; spawns Tokio tasks per effect with operation
  IDs; sends completions through the existing ingress channel.
- `crates/tablerock-cli/src/projection.rs` (new) — core/engine types →
  TUI projection types.
- `crates/tablerock-cli/src/run.rs` — construct executor, hand it the ingress
  sender (fix the dropped-sender gap), route `Update.effect` to it.
- Tests + evidence docs + architecture-doc touch-ups where they describe the
  Phase-1-only shell.

**Out of scope**:
- Any real screen content (plan 006+). This plan proves the machinery with
  ONE thin vertical: profile list load (persistence) and a health-check
  effect (engine) rendered as plain status text in the existing placeholder
  panels.
- Multi-effect batching, animation ticks, telemetry.
- Changing the ingress overflow/resync semantics.

## Git workflow

Trunk-only, Conventional Commits, `git commit -s`, push per checkpoint:
(1) TUI vocabulary + submodel split, (2) executor + wiring, (3) vertical
proof + tests.

## Steps

### Step 1: Effect/message vocabulary + submodel split

Extend `Effect` to an operation-scoped request enum, e.g.:

```rust
pub enum Effect {
    Exit,
    LoadProfileList { request_token: u64, filter: ProfileListFilterSpec },
    CheckSessionHealth { request_token: u64, profile: ProfileRef },
}
```

(`ProfileListFilterSpec`/`ProfileRef` are TUI-local plain types; tokens are
plain u64 correlation values minted by the reducer — the reducer owns a
monotonically increasing counter in the model; no clocks.) Add
`Message::Profiles(ProfilesMsg)` with
`ProfilesMsg::ListLoaded { request_token, items: Vec<ProfileRowProjection>, }`
and `ListFailed { request_token, reason: FailureProjection }`. Root reducer
rejects completions whose token doesn't match the newest issued token (stale
rejection at the root, per TEA doc). Split `model.rs` into `model/` with a
`profiles` submodel holding list state (loading/loaded/failed + rows).
Update the architecture test's file list if it hardcodes paths.

**Verify**: `cargo test -p tablerock-tui` → pass (reducer tests for token
staleness + submodel transitions; render still passes).

### Step 2: Executor in `tablerock-cli`

`effects.rs`: `EffectExecutor::new(engine: EngineHandles, persistence: PersistenceActor, ingress: IngressSender)`.
For each `Effect` received from the loop: spawn onto the current-thread
runtime (`tokio::task::spawn_local` — the runtime is current-thread,
`run.rs:103-107`; if `spawn_local` needs a `LocalSet`, wrap `run()` in one) a
task that performs the call and sends exactly one completion message.
`PersistenceActor` calls are synchronous/blocking (bounded 30s internal
timeouts) — run them via `tokio::task::spawn_blocking`. Engine health uses
plan 002's `DriverSession::health` through `EngineService`. Every task sends
its completion through ingress; overflow already maps to resync. Wire in
`run.rs`: replace the dropped sender (`run.rs:123`) with the executor's, and
pass `Update.effect` values to the executor instead of matching only `Exit`.

**Verify**: `cargo test -p tablerock-cli` → pass; new ingress test proves an
effect completion arrives as a root message.

### Step 3: Thin vertical proof

On entering `Screen::Connections` (initial state), the reducer emits
`LoadProfileList`; completion renders row count in the "Workspace"
placeholder panel (e.g. `Profiles: 3`), failure renders the redacted failure
label. Reducer/render tests via `TestBackend` (pattern:
`tests/shell.rs::assert_render_contains`, `shell.rs:264-306`). Add a CLI
integration test with a temp-dir database path proving end-to-end: start loop
with a seeded `PersistenceActor` (create 2 profiles first), run until the
frame contains `Profiles: 2` (drive the loop as `pty_lifecycle.rs` does, or
factor a headless `run_session`-level harness — prefer the latter; PTY only
for lifecycle).

**Verify**: `cargo test -p tablerock-cli -p tablerock-tui` → pass.

### Step 4: Docs/evidence

Evidence doc: executor design (task-per-effect, token staleness, blocking
isolation for persistence), bounds, failure truth (what happens when the
executor's task panics — must surface as a failure message, not a hang; test
it). Update `docs/architecture/application-pattern.md` only if wording marks
subscriptions as unwired.

**Verify**: full command table green.

## Test plan

- Reducer: token staleness (old completion ignored), loading→loaded→failed
  transitions, resize/focus unaffected by submodel growth.
- Executor: completion delivery, panic-in-task → failure message (use a fake
  port), persistence-blocking isolation (health call while list loads).
- End-to-end: seeded-DB vertical (Step 3).
- Pattern exemplars: `tests/shell.rs` (reducer+render), `tests/ingress.rs`
  (channel semantics), `tests/support/mod.rs` (fake ports in engine tests).

## Done criteria

- [ ] `Effect` carries ≥2 real effect kinds with correlation tokens
- [ ] Executor spawns tasks, persistence goes through `spawn_blocking`, completions arrive via ingress
- [ ] Stale completion rejected by root reducer (test)
- [ ] Vertical: seeded DB renders `Profiles: N` end-to-end (test)
- [ ] `tablerock-tui` manifest still lists only `ratatui-core` + `termrock` (architecture test passes)
- [ ] clippy green; evidence doc added; `plans/README.md` updated

## STOP conditions

- You conclude `tablerock-tui` must depend on `tablerock-core` to avoid
  duplicating projection types — that reverses a guarded architecture
  decision (`tests/architecture.rs` manifest rule). STOP and report the
  trade-off; do not change the guard unilaterally.
- The current-thread runtime + `LocalSet` interaction with `catch_unwind`
  (`run.rs:101-119`) breaks panic-restoration PTY tests twice — STOP.
- Any reducer/view file needs an `.await` or IO import — the design is wrong;
  STOP.

## Maintenance notes

- Every later screen adds: submodel + feature message + effects + projections.
  Keep the executor generic (match on `Effect`, no screen knowledge).
- Reviewer: exactly-one-completion-per-effect discipline; no domain type leaks
  into `tablerock-tui`; token counter monotonicity.
- Deferred: engine event subscription streaming (operation progress events)
  — plan 007/009 wire `EngineService::next_update` pumping; this plan only
  needs request/response effects.
