# Native query-workspace extraction

Date: 2026-08-13

Checkpoint: P4h — extract query workbench surface

## Decision

The production query workspace now lives in `QueryWorkbenchView.swift`.
File/caret context, the native SQL editor, functional run/cancel/find/review
controls, Redis overview action, query status, review and file errors, and
result/empty presentation form one query-owned surface.

## Bounds and failure truth

- Editor selection, delayed session-intent persistence, keyboard shortcuts,
  action enablement, Change Ledger fact, query status, result-grid integration,
  and accessibility identifiers moved unchanged.
- `ResultGridWithInspector` became module-internal so the extracted workspace
  can compose it. No public API was added.
- Liquid Glass remains limited to the functional action strip. SQL and result
  content remain opaque native content surfaces.
- Query execution, cancellation, safety review, result formatting, and database
  policy remain behind existing workflows/backend.
- No fixture symbol, generated FFI type, backend import, or Design Lab
  dependency entered the query workspace file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- Query-tab structural/runtime gate passed.
- Result copy, loaded export, and full streaming-export gate passed.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 5,830 to 5,693 lines.

## Remaining work

Extract object/result workbench surfaces, sheets, and narrow AppKit adapters.
Then enforce the Presentation module and remove Release fixture/scripted-backend
membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
