# TUI effect executor and engine/persistence bridge

Date: 2026-07-18

## Checkpoint

Plan 005. Pure TUI gains operation-scoped effects and profile list submodel;
CLI executes effects on Tokio LocalSet and feeds completions through ingress.

## Decision

- `tablerock-tui` stays free of engine/persistence deps: presentation types
  (`ProfileListFilterSpec`, `ProfileRowProjection`, `FailureProjection`) live
  in the TUI; CLI projects core list rows into them.
- `Effect::{LoadProfileList, CheckSessionHealth, Exit}` with reducer-minted
  `RequestToken` (monotonic u64). Stale completions rejected at the root.
- Bootstrap: first resize on `Screen::Connections` emits `LoadProfileList`.
- `EffectExecutor` owns `PersistenceActor` behind `Arc<Mutex<…>>`; list work
  runs on `spawn_blocking`; completions use `try_send_event` into the existing
  ingress channel. Runtime loop wrapped in `LocalSet` for `spawn_local`.
- Persistence path is process-local (`state-<pid>.db`) until multi-process
  ownership is productized.

## Bounds and failure truth

- Persistence list failure → `ProfilesMsg::ListFailed` with redacted label.
- Executor task join error → `task-failed` label (no hang).
- Path lease contention avoided by per-pid files in this checkpoint.

## Evidence

- `cargo test -p tablerock-tui` (bootstrap + stale token unit tests).
- `cargo test -p tablerock-cli` including PTY lifecycle suite.

## Remaining work

- Connection screens (plan 006) consume list rows and health.
- Wire `CheckSessionHealth` through `EngineService` + registry.
- Single shared state.db with explicit multi-instance policy.
