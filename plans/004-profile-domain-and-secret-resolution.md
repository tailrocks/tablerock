# Plan 004: Close the profile-domain gaps — environment tag, group operations, search fields, secret resolution

> **Executor instructions**: Follow step by step; verify each step; STOP
> conditions binding. Update `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat d8b113b..HEAD -- crates/tablerock-core/src/profile*.rs crates/tablerock-core/src/secret.rs crates/tablerock-persistence`
> Compare "Current state" excerpts on any change; mismatch = STOP.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED (schema migration + secret handling)
- **Depends on**: plans/001-ci-verification-baseline.md (independent of 002/003)
- **Category**: direction
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

Phase 3 (`docs/product/connections.md`) requires four things the profile
domain cannot express today: (1) a first-class **environment tag** with
mandatory production-warning semantics; (2) **group operations** (rename,
delete-moves-to-ungrouped) that today would require N unrelated CAS rewrites;
(3) list **search over host and database**, which the spec demands but the
store's search skips; (4) **secret resolution** — the model stores references
but no code can turn `PromptOnConnect` or `DangerousPlaintext` into bytes for
a connection, and connections.md requires "a profile with an unresolved or
failing secret source fails before any network I/O". Plan 006 (connection
screens) is blocked on all four.

## Current state

- `crates/tablerock-core/src/profile_aggregate.rs:83` — `ProfileOrganization`:
  single optional `ProfileGroupName`, up to 32 generic `ProfileTag`s,
  `favorite`, `saved_order`. **No environment-tag field.** Product spec
  requires exactly one optional tag: `production/staging/development/testing`
  or custom label, rendered with label+color never color alone, production as
  persistent warning (`docs/product/connections.md` "Environment tags").
- `crates/tablerock-persistence/migrations/0003-saved-profiles.sql` —
  `saved_profiles.group_name` nullable string; no groups table; no
  environment column.
- `crates/tablerock-persistence/src/profile_store.rs:509-518` — list search
  matches name/group/tags only; host/database (`DefaultContext` property)
  excluded. Spec: "Search filters by name, host, database, and group."
- Secrets (`crates/tablerock-core/src/secret.rs`): `SecretSourceKind` with 5
  variants; `DangerousPlaintext` zeroizes on drop (`secret.rs:257-293`);
  **no resolution code anywhere** (verified by survey: no 1Password/keychain/
  env/prompt lookups in the workspace).
- Persistence actor API (`crates/tablerock-persistence/src/lib.rs:53-160`):
  `open/health/create_profile/get_profile/replace_profile/delete_profile/list_profiles/shutdown`,
  bounded 32-command channel, migrations 0001–0006 run at open with
  prefix-validated ledger (`lib.rs:394-433`).
- Fixed decisions binding this plan (`docs/architecture/fixed-decisions.md`
  "Secret model", "Password staging (revision 2026-07-18)"): first delivery is
  **prompt-on-connect + acknowledged plaintext only**; 1Password/Keychain/env
  staged later; resolved bytes never enter stable state, logs, events;
  `SecretSource` model already carries all variants — delivery order changes,
  not the model.
- Convention: every schema change ships versioned migration + fixtures +
  migration doc (`crates/tablerock-persistence/migration-docs/`,
  `MIGRATING.md`) in the same checkpoint (delivery-plan invariant 11).

## Commands you will need

| Purpose | Command | Expected |
|---|---|---|
| Core tests | `cargo test -p tablerock-core` | pass |
| Persistence tests | `cargo test -p tablerock-persistence` | pass |
| Build/lint | `cargo check --workspace --all-targets && cargo clippy --workspace --all-targets` | exit 0 |

## Scope

**In scope**:
- `crates/tablerock-core/src/profile_aggregate.rs` — new
  `EnvironmentTag` enum (`Production | Staging | Development | Testing | Custom(ProfileLabel)`)
  with `is_production_like()` (Custom never warns unless… no: only `Production`
  warns; keep it simple and spec-exact), added to `ProfileOrganization` as
  `environment: Option<EnvironmentTag>`; bump nothing — extend within
  `SCHEMA_VERSION 1` only if `from_wire` compatibility is preserved by making
  the field optional-with-default, otherwise bump `ProfileAggregate::SCHEMA_VERSION`
  to 2 with explicit `from_wire` acceptance of 1 (state the choice in the
  migration doc).
- `crates/tablerock-core/src/profile_list.rs` — `ProfileListFilter` gains
  `environment: Option<EnvironmentTag>`; `ProfileListItem` projects the tag;
  search-term matching extended to host/default-context literals (core side:
  accept candidate strings supplied by the store).
- `crates/tablerock-persistence/migrations/0007-environment-tag.sql` (new
  column + index) and `0008-…` if group operations need one (they do not — see
  Step 3 design).
- `crates/tablerock-persistence/src/profile_store.rs` — encode/decode the tag;
  extend search to include host/default-context literal properties (secret
  sources remain unsearchable by design); new group operations.
- `crates/tablerock-persistence/src/lib.rs` — actor commands:
  `rename_group(old, new)`, `delete_group(name)` (moves members to ungrouped),
  `list_groups()`; each transactional.
- New crate module `crates/tablerock-engine/src/secret_resolution.rs` (or a
  core-adjacent module in `tablerock-engine`; NOT in `tablerock-core`, which
  must stay IO-free per its architecture test) — `resolve(source, prompt_port) -> Result<ResolvedSecret, _>`
  for `PromptOnConnect` (via an injected prompt port trait) and
  `DangerousPlaintext` (copy out of the aggregate). `ResolvedSecret` wraps
  zeroizing bytes, `Debug`-redacted, non-Clone.
- Tests, migration docs, evidence docs, parity-ledger rows ("Profile
  organization", "Environment tag"), roadmap notes.

**Out of scope**:
- 1Password (`op read`), Keychain, environment-variable resolution — staged
  later by fixed decision; the `SecretSource` variants stay untouched.
- Any UI (plan 006).
- Live health/last-used tracking (needs engine sessions — plan 006 wires it).
- `support_facts` table cleanup (dormant; leave).

## Git workflow

Trunk-only, Conventional Commits, `git commit -s`, push per checkpoint:
(1) environment tag core+migration, (2) group ops, (3) search fields,
(4) secret resolution.

## Steps

### Step 1: `EnvironmentTag` end-to-end

Core type + `ProfileOrganization.environment` + list filter/projection;
migration `0007` adds nullable `environment_kind` (1–4) +
`environment_label` (custom text, CHECK bounds) + index for filtering;
encode/decode in `profile_store.rs` (follow the existing enum↔u8 mapping
style around `profile_store.rs:1058`). Migration doc explaining the
versioning choice. Tests: round-trip, custom-label bounds, filter, decode of
pre-0007 rows (NULL → None).

**Verify**: `cargo test -p tablerock-core -p tablerock-persistence` → pass.

### Step 2: Group operations

Actor commands `rename_group`/`delete_group`/`list_groups` implemented as
single transactions in `profile_store.rs` (`UPDATE saved_profiles SET group_name = … WHERE group_name = …`),
bypassing per-profile revision CAS **deliberately**: group membership is
organization metadata, not connection identity. Record that decision in the
evidence doc. `delete_group` sets members' `group_name` NULL (spec: never
cascade-delete). Return affected counts. Tests: rename with 0/N members,
delete moves members, concurrent-CAS profiles unaffected, transactionality
under mid-operation error (fault-injection style used in
`tests/crash_recovery.rs`).

**Verify**: `cargo test -p tablerock-persistence` → pass.

### Step 3: Search over host/database

Extend the store's candidate matching (`profile_store.rs:509-518`) to include
the `Host` and `DefaultContext` property **literals only** (properties with a
secret source are never searched or exposed). Tests: search by host hits,
search by database hits, secret-sourced host is not searchable, case/NFKC
folding matches the existing `ProfileSearchTerm` behavior.

**Verify**: `cargo test -p tablerock-persistence` → pass.

### Step 4: Secret resolution (prompt + plaintext)

New engine-side module with:

```rust
pub trait SecretPromptPort: Send {
    fn request(&mut self, field: SecretField, profile: ProfileName) -> Result<ResolvedSecret, SecretResolutionError>;
}
pub fn resolve_for_connect(binding: &ProfilePropertyBinding, prompt: &mut dyn SecretPromptPort)
    -> Result<Option<ResolvedSecret>, SecretResolutionError>;
```

Rules (all testable): resolution happens only when a connect/test actually
needs the field; unsupported kinds (`OnePassword`, `HostEnvironment`,
`Keychain`) return `SecretResolutionError::SourceNotYetSupported { kind }` —
fail-closed **before any network I/O**; `ResolvedSecret` zeroizes on drop,
Debug prints byte count only, cannot be cloned or serialized. Tests assert:
plaintext round-trip, prompt port called exactly once per request, unsupported
kinds fail closed, `format!("{:?}", resolved)` contains no secret bytes.

**Verify**: `cargo test -p tablerock-engine --lib` → pass.

### Step 5: Docs/evidence/ledger

Evidence doc per checkpoint; parity ledger rows "Environment tag" and
"Profile organization" updated; `docs/product/connections.md` untouched
(spec already matches); `MIGRATING.md` + migration-docs entry for 0007.

**Verify**: full command table green.

## Test plan

Model persistence tests after `crates/tablerock-persistence/tests/actor.rs`
and `profiles.rs` (existing CRUD/CAS suites); core tests after
`tablerock-core/tests/profile_aggregate.rs`. New coverage list: environment
round-trip + filter + legacy-row decode; group rename/delete semantics;
host/database search incl. secret-source exclusion; resolution fail-closed
matrix; zeroize/redaction assertions.

## Done criteria

- [ ] `EnvironmentTag` persisted, filterable, projected in `ProfileListItem`
- [ ] `rename_group`/`delete_group`/`list_groups` on `PersistenceActor`; delete moves members to ungrouped
- [ ] Search matches host + database literals; secret-sourced values excluded (test proves)
- [ ] `resolve_for_connect` handles Prompt + DangerousPlaintext; other kinds fail closed pre-I/O
- [ ] Migration 0007 applies on fresh AND existing DBs (test opens a pre-0007 fixture)
- [ ] clippy green; evidence + migration docs + ledger updated
- [ ] `plans/README.md` row updated

## STOP conditions

- `ProfileAggregate` schema versioning cannot absorb the new field without
  breaking `from_wire(1)` round-trips of existing rows — STOP and report the
  versioning options (this is a compatibility decision).
- Any test requires weakening the "literal forbidden for Password/private-key"
  policy (`profile.rs:34`) — STOP.
- You find existing persisted data would be silently reinterpreted — STOP.

## Maintenance notes

- Plan 006 consumes: tag in list/editor/context bar; group CRUD in the list
  screen; `resolve_for_connect` inside its Test/Connect effects.
- Env resolution: evidence 336. 1Password `op read`: evidence 337.
  Keychain remains `SourceNotYetSupported` until native adapter (plan 020).
- Reviewer: migration idempotence, transactional group ops, zero secret bytes
  in any Debug/log path (grep for `{:?}` on resolved types).
