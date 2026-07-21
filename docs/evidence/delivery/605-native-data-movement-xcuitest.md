# Native data-movement user-control tests

Date: 2026-07-22

## Requirement

Plan 021 requires XCUITest to operate file import/export controls. Existing
live bridge audits proved Rust CSV and export semantics, but they initialized
the workflow programmatically and did not prove native menu, sheet, review, or
apply controls.

## Implementation

- Test mode replaces system file panels with an injected path port. Both open
  and save paths must resolve beneath that test's isolated data root; lexical
  escapes and symlink-parent escapes fail closed.
- Scripted success backend exposes deterministic CSV export and consume-once
  import review/apply behavior for presentation tests. It does not claim real
  database semantics.
- Stable identifiers cover export menu/format/outcome and CSV
  open/sheet/stage/apply/outcome controls.
- XCUITest exports a loaded result through the menu and verifies exact bytes,
  then opens a CSV through the object toolbar, stages review, applies it, and
  observes the terminal outcome. Each test owns and removes a unique root.
- App-unit coverage proves inside-root paths work while outside-root and
  symlink escapes are rejected.

## Verification

- `swiftc -parse native/Sources/TableRockApp/TableRockApp.swift native/Tests/TableRockAppTests/BridgeModelScenarioTests.swift native/Tests/TableRockAppUITests/TableRockAppUITests.swift`
- `swiftc -typecheck -parse-as-library -swift-version 6 -strict-concurrency=complete -warnings-as-errors ... native/Sources/TableRockApp/*.swift`
- `git diff --check`
- Hosted run 29864380153 passed loaded-result export and exposed the import
  sheet's Stage action as clipped below the fixed-height sheet viewport. Import
  details now scroll independently while Stage/Apply/Discard remain in a fixed
  action footer. Exact-main hosted proof remains required.

## Provenance

TablePro establishes only that native import/export workflows exist. No source,
test, identifier, wording, screenshot, layout measurement, color, asset, or key
binding was copied or translated. Controls follow TableRock's product and
plan-021 requirements.
