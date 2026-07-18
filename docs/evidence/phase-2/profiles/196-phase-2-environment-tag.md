# Phase 2 environment tag

Date: 2026-07-18

## Checkpoint

Plan 004 step 1. Profiles carry an optional environment tag end-to-end:
core organization, list filter/projection, migration 0007, and store encode.

## Decision

- `EnvironmentTag`: Production | Staging | Development | Testing | Custom(ProfileTag).
- Only `Production` is `is_production_warning()`.
- `ProfileOrganization.environment: Option<EnvironmentTag>`; empty requires
  `None`. Aggregate schema stays **1** (organization was already column-backed).
- Migration 0007: nullable `environment_kind` / `environment_label` + index.
- Pre-0007 rows decode as `None`.

## Bounds and failure truth

- Custom requires kind 5 + 1..=64 byte label; fixed kinds require NULL label.
- Invalid wire kinds fail decode (`ProfileDecode` / `InvalidEnvironment`).
- List filter matches kind (+ label for custom).

## Evidence

- `cargo test -p tablerock-core -p tablerock-persistence`
- Migration ledger schema version 7 in actor/crash/profile_create tests.

## Remaining work

- Group rename/delete/list (step 2).
- Host/database search (step 3).
- Secret resolution prompt/plaintext (step 4).
