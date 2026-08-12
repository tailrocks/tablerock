# Native Redis workflow extraction

Date: 2026-08-13

Checkpoint: P3k — extract Redis administration workflows

## Decision

Redis overview loading and Pub/Sub subscription start, polling, cancellation,
and close behavior now live in `WorkbenchPresentationStore+Redis.swift`.
The polling task remains main-actor owned and cancels when the sheet closes or
the subscription reaches a terminal phase.

## Bounds and failure truth

- Engine/session guards, selector validation, polling cadence, cancellation,
  status projection, and error messages moved unchanged.
- Redis loading/error/subscription state and the polling task became
  module-internal for same-type cross-file access. No public API was added.
- Redis operations remain behind `WorkbenchBackend`; no protocol or database
  behavior moved into Swift presentation.
- No fixture symbol, generated FFI type, or Design Lab dependency entered the
  Redis extension.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures, including Pub/Sub message, gap,
  cancellation, and terminal-state assertions.
- Redis overview structural/runtime gate passed.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.

The focused Pub/Sub UI-host test was also attempted. It reached the production
window and menu, then failed because the menu command was disabled before the
sheet opened. The behavior-level Pub/Sub test passed. This matches the existing
command-host/window-store isolation debt observed before this checkpoint; the
extraction did not change command enablement. Final migration must remove that
dual-host condition before the complete UI suite can be green.

## Remaining work

Extract PostgreSQL administration and safety workflows. Then enforce the
Presentation module and remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
