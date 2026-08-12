# Native Redis-sheet extraction

Date: 2026-08-13

Checkpoint: P4q — extract Redis operational presentation

## Decision

Bounded Redis INFO overview and Pub/Sub subscription sheets now live in
`RedisSheets.swift`. The Redis verifier now asserts the declaration contract
without encoding obsolete file-private access.

## Bounds and failure truth

- Refresh, loading and empty states, bounded INFO facts, subscribe/pattern
  modes, cancellation, delivery-gap truth, retained-message window, and active
  subscription dismissal guard moved unchanged.
- Redis sampling, subscription lifetime, bounds, discontinuity accounting, and
  failure outcomes remain below presentation.
- No backend, fixture, generated FFI, AppKit, or Design Lab dependency entered
  the Redis file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- `scripts/verify-native-redis-overview.sh` passed its shared adapter and live
  Redis runtime gate after the ownership-aware declaration assertion.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 4,105 to 3,960 lines.

## Remaining work

Extract review, PostgreSQL, inspector, tab, toolbar, and settings surfaces.
Then enforce the Presentation module and remove Release fixture/scripted-
backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
