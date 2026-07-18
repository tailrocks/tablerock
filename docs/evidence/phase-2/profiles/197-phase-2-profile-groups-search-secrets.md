# Phase 2 profile groups, host search, and secret resolution

Date: 2026-07-18

## Checkpoint

Plan 004 steps 2–4.

## Decision

### Group operations

`PersistenceActor::{rename_group, delete_group, list_groups}` run as single
SQL transactions on `saved_profiles.group_name` without per-profile revision
CAS. Group membership is organization metadata, not connection identity.
`delete_group` nulls members (never cascade-deletes profiles).

### Host/database search

List search matches name, group, tags, and **literal** Host / DefaultContext
property text only. Secret-sourced host/context values are not selected into
the search projection (`source_kind = 1` only).

### Secret resolution

`tablerock-engine::secret_resolution`:
- `resolve_for_connect` for `PromptOnConnect` (via `SecretPromptPort`) and
  `DangerousPlaintext` (copy bytes).
- `OnePassword` / `HostEnvironment` / `Keychain` →
  `SourceNotYetSupported` fail-closed before network I/O.
- `ResolvedSecret` zeroizes on drop; Debug shows field + length only; not Clone.

## Evidence

- `cargo test -p tablerock-persistence`
- `cargo test -p tablerock-engine --lib secret_resolution`
- `cargo clippy -p tablerock-engine -p tablerock-persistence --all-targets`

## Remaining work

- Plan 006 connection screens consume tags, groups, and resolution.
- Later stages implement 1Password/Keychain/env resolvers.
