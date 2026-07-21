# Native result paging XCUITest

Date: 2026-07-21

## Claim

The shipped native result surface exposes bounded continuation through a real
control. A deterministic first page contains exactly 500 rows and a stable
result identity/revision. Clicking `Load more rows` crosses the production
`WorkbenchBackend.fetchPage` boundary, appends row 501 without replacing the
resident page, and removes the continuation control after the short terminal
page.

## Automated proof

- `TableRockAppUITests.testResultPagingAppendsThroughUserControl` launches the
  shipped app with an isolated scripted backend, observes the 500-row summary,
  clicks `results.next-page`, observes the 501-row summary, and verifies that
  no further continuation remains.
- The fixture uses the same `BridgeModel.loadMore`, table append validation,
  result identity, revision, and AppKit grid path as production.

## Verification

- `git diff --check` passes.
- Local SwiftPM XCTest is unavailable because this host has Command Line Tools
  only (`xcode-select -p` is `/Library/Developer/CommandLineTools`).
- XCUITest execution remains pending on the hosted Xcode 26.6 checkpoint after
  push.

## Provenance

TablePro establishes only that paged result navigation is a broad workbench
workflow. TableRock's 500-row boundary, identifiers, wording, state model,
layout, implementation, and test are independently defined from repository
requirements and direct tests. No external source, screenshot, asset,
measurement, product text, color, or key binding was copied or translated.
