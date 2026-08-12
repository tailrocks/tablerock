# Native PostgreSQL workflow extraction

Date: 2026-08-13

Checkpoint: P3l — extract PostgreSQL administration workflows

## Decision

PostgreSQL activity and backend signalling, relationship navigation, role and
privilege review, and supervised backup/restore tool lifecycle now live in
`WorkbenchPresentationStore+Postgres.swift`.

The presentation store remains the state owner. Database discovery, authority,
review-token consumption, signalling, and subprocess supervision remain behind
the bridge-neutral backend.

## Bounds and failure truth

- Engine/session guards, typed projections, review consumption, refreshes,
  file-panel requests, security-scope lifetime, polling, cancellation, and
  error messages moved unchanged.
- PostgreSQL loading/error fields and tool security-scope state became
  module-internal for same-type cross-file access. No public API was added.
- Rust retains database and safety behavior. Swift only orchestrates confirmed
  presentation flows through `WorkbenchBackend`.
- No fixture symbol, generated FFI type, or Design Lab dependency entered the
  PostgreSQL extension.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- Focused behavior tests passed for typed activity/signalling, relationship
  navigation, role review/apply, and supervised backup status.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.

## Remaining work

Extract safety and editor workflows. Then enforce the Presentation module and
remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
