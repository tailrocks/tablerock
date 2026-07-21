# Native checkpoint control reachability

## Finding

Native checkpoint run 29864380153 passed the Rust bridge, generated bindings,
Swift package tests, and fourteen of sixteen UI tests. It then proved two
control-boundary defects:

- the CSV review action remained inside a scrolling content region and was not
  exposed as a reachable button after staging;
- a result cell exposed accessibility metadata but had no cell-owned activation
  path, so an accessibility click did not reliably drive selection or open the
  value inspector.

The architecture permitted both defects because reachability and activation
were incidental consequences of layout and `NSTableView.clickedColumn`. The
controls had identifiers, but the owning views did not guarantee their user
actions independently of scrolling or table hit-test state.

The live bridge proof had a related evidence weakness: one anonymous executable
contained several protocol behaviors, so CI could not identify the exact
engine/behavior assertion that failed.

## Correction

- Keep the staged CSV apply/discard controls outside the scrolling preview and
  review content. They remain visible and actionable while a review exists.
- Give every result cell an explicit click and accessibility-press action. The
  cell records its column, selects its row, and invokes the same value-selection
  callback without depending on `clickedColumn`.
- Replace the anonymous `BehaviorProof` executable with named Swift XCTest
  cases for typed pages, catalog browse, PostgreSQL cancellation, and reviewed
  apply. The nightly workflow runs each case against isolated PostgreSQL,
  ClickHouse, and Redis containers and archives the named-test log.
- Preserve decode performance and leak measurement in a dedicated
  `PageDecodeBenchmark` executable with no behavioral-test branching.

No test assertion, timeout, or product behavior was weakened.

## Verification

- Native checkpoint run 29864380153: exact failure evidence at
  `TableRockAppUITests.swift:153` and `:259`.
- XcodeGen 2.46.0 regeneration is clean.
- `git diff --check` and `bash -n scripts/verify-native-behavior.sh
  scripts/verify-native-page-performance.sh` pass.
- `./scripts/build-native-app.sh` passes with Swift 6 strict concurrency and
  warnings as errors.
- Local SwiftPM XCTest execution is unavailable because this host has Command
  Line Tools only; the canonical macOS 26/Xcode 26.6 checkpoint is the required
  execution proof after push.

## Provenance

TablePro establishes only the broad workflows of native result inspection and
data movement. No external source, test, identifier, wording, screenshot,
layout measurement, color, asset, or key binding influenced this correction.
The changes derive from TableRock plan 021 and hosted XCUITest evidence.
