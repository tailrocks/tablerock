# Workbench shell hierarchy pass

Date: 2026-08-09

## Path

```text
Session open (Rust / UniFFI)
  → BridgeModel.sessionHex + connectedEngine + profile env/safety
  → WorkbenchShellView | WorkbenchWelcomeView
  → context strip · tab strip · Query/Object workbench · status bar
```

## Before

Connected detail was still a **vertical-slice demo stack**:

* marketing title “TableRock” and tertiary footer;
* green “Connected · session …” as primary chrome;
* `Browse catalog` competing with sidebar;
* tabs + content not filling the detail pane;
* no permanent status bar matching `workbench.md`.

That fails the expert-workbench bar: content must dominate; context must be dense.

## Design decision

One coherent direction: **real workbench shell**.

| Region | Role | Material |
|---|---|---|
| `WorkbenchContextStrip` | Connection · engine · short session · Halo · catalog refresh | Dense chrome; glass catalog button |
| `QueryTabStrip` | Tabs | Selected glassProminent / unselected plain |
| Content | SQL+grid / object data | Opaque; fills remaining space |
| `WorkbenchStatusBar` | READY/RUNNING/ERROR + facts + production word | Permanent footer; monospaced digits |
| Welcome | Direct connect only | No fake workbench chrome |

Fixed: shell regions. Moves: only tab content. Permanent: status bar. Progressive: catalog errors in status facts; sheets for secondary tools.

## Research principles

* Xcode / professional multi-pane tools: fixed chrome, fill content.
* Things/Craft: no permanent marketing chrome in working mode.
* Git clients: status bar for operation truth.
* Product `workbench.md` layout diagram as authority.
* Liquid Glass: chrome only (toolbar + glass buttons), not content.

## Implementation

* `TableRockApp.swift` — `WorkbenchShellView`, `WorkbenchWelcomeView`,
  `WorkbenchContextStrip`, `WorkbenchStatusBar`; detail branch on `sessionHex`.
* `WorkbenchTypes.swift` — `WorkbenchStatusFacts.line` pure assembly.
* Tests: `WorkbenchStatusFactsTests.swift`.

## Validation

```text
cd native && swift build --target TableRockFeature
cd native && swift build --target TableRockApp
```

Headless: no interactive light/dark screenshots; compile is the gate.

## Remaining weaknesses

* Database/schema pickers still not in context strip (toolbar future).
* Ledger pending count not yet on status bar (needs staged draft projection).
* Sidebar still dual-mode (profiles + catalog under connection list).

## Next improvement for Workbench

Add **database/schema selectors** to the context strip from Rust session facts
(when available), matching the product context-bar contract.
