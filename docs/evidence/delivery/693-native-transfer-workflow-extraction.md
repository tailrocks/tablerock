# Native transfer workflow extraction

Date: 2026-08-13

Checkpoint: P3j — extract result and CSV transfer workflows

## Decision

Result copy, loaded/full export, streaming-export lifecycle, CSV preview,
reviewed CSV apply, cancellation, and error copying now live in
`WorkbenchPresentationStore+Transfers.swift`.

Object-result pagination moved to the navigation extension. Workspace-intent
persistence moved beside restoration. These placements keep transfer behavior,
navigation behavior, and session restoration independently owned without
changing the single presentation-state owner.

## Bounds and failure truth

- Multi-representation pasteboard payloads, security-scoped file access,
  streaming progress, cancellation, CSV review-token handling, and error
  messages moved unchanged.
- Transfer operation identifiers and selected CSV URL became module-internal for
  same-type cross-file access. No public API was added.
- The fixture-only stream-export polling helper became module-internal because
  primary-declaration fixture setup still invokes it. That Release fixture debt
  remains frozen and enforced by the source-ownership gate.
- Formatting, export, transactional import, and review-token authority remain
  behind `WorkbenchBackend`; Rust keeps data and safety ownership.
- No fixture symbol, generated FFI type, or Design Lab dependency entered the
  transfer extension.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- Result copy, loaded export, and full streaming-export gate passed.
- CSV preview, reviewed apply, and live PostgreSQL transaction gate passed.
- Multi-window structural/runtime gate passed after persistence relocation.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.

## Remaining work

Extract administration and safety workflows. Then enforce the Presentation
module and remove Release fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
