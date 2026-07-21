# Native control accessibility contracts

Date: 2026-07-22

## Failure class

Native checkpoint run 29854847292 compiled the application and passed 11 of
14 XCUITests. The remaining tests exposed three product-contract defects:

- a query-tab menu action had no stable accessibility identifier;
- the AppKit result cell's role was less stable than its identifier;
- the paging action lived below the minimum-height result grid and could be
  outside the workbench viewport.

The earlier gesture workaround in evidence 597 did not make paging reliably
hittable and is superseded by this correction.

## Correction

- Query-tab `Close` actions expose `query.tab.close`; UI automation now selects
  the semantic identifier without assuming an AppKit menu-item role.
- Result-cell automation observes the existing `results.cell.<row>.<column>`
  identifier without assuming whether AppKit exports the cell or its text field.
- `Load more rows` moved into the always-visible result toolbar. Result paging
  remains adjacent to its result while no longer depending on synthetic scroll
  gestures or viewport dimensions.

These identifiers describe user-facing actions and data cells. They do not add
fixture-only production controls.

## Verification

- `swiftc -parse native/Sources/TableRockApp/TableRockApp.swift native/Tests/TableRockAppUITests/TableRockAppUITests.swift`
- `git diff --check`
- Exact-main hosted Xcode checkpoint remains required after push.

## Provenance

No external product source, screenshot, text, measurement, color, asset, or key
binding influenced this correction. It derives from TableRock's native
accessibility contract and hosted XCUITest evidence.
