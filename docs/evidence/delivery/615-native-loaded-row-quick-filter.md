# Native loaded-row quick filter

Date: 2026-07-22

## Requirement

Native result grids need per-tab resident filtering that can never be confused
with PostgreSQL/ClickHouse server filtering.

## Delivery

- Added independent quick-filter text to query and object tabs.
- The result toolbar labels the field `Filter loaded rows` and continuously
  announces `Loaded rows only · visible/loaded`.
- Matching is case- and diacritic-insensitive across display cells. It changes
  only the resident projection and performs no I/O.
- Filtered row selection maps back to the source row before opening the value
  inspector, preserving column metadata and typed cell bytes.
- Added a deterministic three-row XCUITest fixture that types a name, observes
  one visible row, verifies the cell value, and checks the explicit 1/3 label.

## Verification

```text
./scripts/build-native-app.sh
# strict Swift 6 direct app build passed

xcrun swiftc -parse native/Tests/TableRockAppUITests/TableRockAppUITests.swift
# parsed
```

Hosted run `29875658677` found the control and status element but the status
exposed an empty accessibility label. The status now explicitly publishes the
same loaded-only text as its visible and accessibility label. Exact-main
hosted proof remains required.

## Remaining scope

Durable object restoration, column layout controls/persistence, and staged
native edits remain open.

## Documentation and provenance

No external product source, tests, identifiers, product text, assets,
screenshots, layout measurements, colors, or key bindings influenced this
resident-only projection. It derives from TableRock's existing grid contract
and direct tests.
