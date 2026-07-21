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
- Result cells alone own `results.cell.<row>.<column>`; their non-actionable
  text children no longer duplicate the identifier. AppKit cells explicitly
  expose the accessibility-element and cell-role contract rather than relying
  on framework defaults.
- `Load more rows` moved into the always-visible result toolbar. Result paging
  remains adjacent to its result while no longer depending on synthetic scroll
  gestures or viewport dimensions.
- The discard confirmation action owns `query.tab.discard-close`, independent
  of duplicate visible-label representations emitted by SwiftUI.
- Shared count formatting preserves singular `column`/`row` status text after
  page append and in other native result summaries.

These identifiers describe user-facing actions and data cells. They do not add
fixture-only production controls.

## Verification

- `swiftc -parse native/Sources/TableRockApp/TableRockApp.swift native/Tests/TableRockAppUITests/TableRockAppUITests.swift`
- `git diff --check`
- Hosted run 29857152743 proved the paging control itself became hittable and
  appended row 501. It then exposed duplicate AX ownership, a duplicate
  confirmation-label query, and `1 columns`; those follow-up corrections await
  exact-main hosted proof after push.
- Hosted run 29859429440 passed 13 of 14 XCUITests, including dirty close and
  paging. Its remaining grid test showed that removing the child identifier
  also removed the implicit cell from the AX tree; explicit cell-role ownership
  is the follow-up under test.

## Provenance

No external product source, screenshot, text, measurement, color, asset, or key
binding influenced this correction. It derives from TableRock's native
accessibility contract and hosted XCUITest evidence.
