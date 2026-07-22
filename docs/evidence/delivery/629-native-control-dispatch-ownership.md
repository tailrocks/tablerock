# Evidence 629: native control-dispatch ownership

Date: 2026-07-22

## Authoritative narrowing

Native Checkpoint run `29881771295` tested `f6591b0` after connected-workbench
containment. It passed loaded-result export, loaded-row quick filtering, and
result paging. Two XCUITests remained red:

- `testCsvImportReviewsAndAppliesThroughUserControls` could not resolve
  `import.csv.stage`;
- `testGridSelectionOpensValueInspector` clicked `results.cell.0.0`, but the
  inspector did not appear.

All Rust, generated-binding, Swift model, universal XCFramework, and other 17
XCUITest cases passed up to the Xcode-plan failure.

## Root cause and fix

CSV identifiers were followed by `.disabled`, so the final styled control
wrapper did not own the stable identifier. Stage, Apply, and Discard now apply
their identifiers last.

AppKit result cells used an unbordered `NSButton` targeting itself. XCUITest
resolved and clicked the button, but target/action dispatch did not reach the
selection closure. `ResultCellButton` now owns physical activation through
`mouseDown(with:)`, parallel to its existing accessibility press action. The
table coordinator remains the single selection owner; the button only forwards
one activation fact.

## Verification

```text
cd native && rtk swift build -c release
ok (build complete)

swiftc -parse native/Sources/TableRockApp/TableRockApp.swift
exit 0

mise exec -- rtk cargo test -p tablerock-core --test screen_manifest
cargo test: 1 passed (1 suite, 0.00s)

mise exec -- rtk cargo clippy -p tablerock-core --test screen_manifest -- -D warnings
cargo clippy: No issues found

rtk git diff --check
exit 0
```

Hosted XCUITest remains required after push.
