# Native dirty-tab close XCUITest

Date: 2026-07-21

## Claim

The shipped native query-tab surface does not silently discard edited SQL.
Closing a dirty tab through its real action menu presents an explicit choice;
only `Discard and Close` removes the tab and selects the remaining tab.

## Automated proof

`TableRockAppUITests.testDirtyQueryTabRequiresDiscardConfirmation` launches
the shipped app with two isolated query tabs, edits the selected AppKit query
editor, invokes that tab's action menu, requests Close, verifies both discard
and cancel choices, confirms discard, and observes that the dirty tab is gone
while the other tab remains.

The test drives the production editor, menu, `requestCloseQueryTab`, SwiftUI
confirmation dialog, and tab-removal path. It does not call the model directly.

## Verification

- `git diff --check` passes.
- Local XCTest remains unavailable because this host has Command Line Tools
  only.
- XCUITest execution remains pending on the hosted Xcode 26.6 checkpoint after
  push.

## Provenance

TablePro establishes only that multi-tab database workbenches and unsaved
editor safeguards are broad workflows. TableRock's identifiers, wording,
state model, layout, implementation, and tests are independently defined from
repository requirements and direct tests. No external source, screenshot,
asset, measurement, product text, color, or key binding was copied or
translated.
