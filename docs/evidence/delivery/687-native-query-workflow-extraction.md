# Native query workflow extraction

Date: 2026-08-13

Checkpoint: P3d — extract query execution and review workflows

## Decision

Query submission, parameterized execution, cancellation, paging, catalog
browse, explain, probe Change Review, and query/DDL copy actions now live in
`WorkbenchPresentationStore+Queries.swift` as a MainActor-isolated extension.

The primary store file now owns stored state and orchestration outside this
workflow. The extension still operates on the same single window-owned
Observation object; no second store or component-owned application state was
introduced.

## Bounds and failure truth

- Method bodies, ordering, operation IDs, page-envelope handling, cancellation
  projection, review-token behavior, and copy payloads moved unchanged.
- Six stored collaborators and two mutable status values became
  module-internal so the same-type extension can access them across files. No
  public API was added; the compiler-enforced Presentation module checkpoint
  will restore a narrower external surface.
- `fetchPage` became module-internal because deterministic initialization
  proofs invoke it from the primary store file. Its signature and behavior did
  not change.
- No FFI, bridge, fixture symbol, persistence, or credential implementation
  entered the query extension.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- Structural and runtime gates passed for query tabs, object tabs/catalog
  browse, and result copy plus loaded/full export.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.

The first direct compile identified exactly six file-private collaborators and
two private setters required by the same-type extension. Only those accessors
were widened to module scope; the second compile passed.

## Remaining work

Extract connection/profile, tab/navigation, transfer, administration, and
restoration extensions. Keep stored state centralized until those workflow
boundaries are proven.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
