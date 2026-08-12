# Native connection workflow extraction

Date: 2026-08-13

Checkpoint: P3e — extract connection and profile workflows

## Decision

Profile groups, ordering, favorites, URL review, quick switching, profile
editing, Keychain handoff, direct and saved connection, health checks,
reconnection, password prompts, and connection-state presentation now live in
`WorkbenchPresentationStore+Connections.swift`.

The MainActor extension mutates the existing single window-owned store. Rust
and the bridge-neutral `WorkbenchBackend` protocol remain authoritative for
connection and database behavior; Swift continues to own native form and
navigation state only.

## Bounds and failure truth

- Method bodies, secret zeroization, Keychain cleanup, session replacement,
  health/reconnect generations, prompt rules, restoration calls, and failure
  strings moved unchanged.
- Seven stored collaborators, three status setters, and two existing store
  helpers became module-internal for same-type cross-file access. No public API
  or second state owner was added.
- Fixture selection remains a typed immutable input; no Release fixture symbol
  appears in the connection extension.
- Query execution remains in its separate extension; live FFI construction
  remains private to `TableRockBridge`.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- Structural and runtime gates passed for profile editing, profile groups,
  multi-window ownership, and query tabs after connection replacement.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.

The first direct compile identified the precise cross-file collaborators and
setters. Only those members were widened to module scope; the second compile
passed.

## Remaining work

Extract navigation/tabs, transfer, administration, and restoration workflows.
Then enforce the Presentation module so module-internal store details remain
inaccessible to the application target.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
