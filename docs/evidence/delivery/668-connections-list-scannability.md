# Connections list scannability pass

Date: 2026-08-09

## Path

```text
Persistence profiles → UniFFI list_profiles → WorkbenchProfileItem
  → BridgeModel.profileSections + connectionState()
  → ConnectionsProfileList / ProfileRow → SwiftUI List
```

## Before

* Profile rows were text-heavy without a stable engine scan column.
* Live state mixed sentence case with no word/detail split.
* Active session profile was not visually marked in the list.
* When connected, catalog was **stacked under** the list with a fixed min height,
  starving the connections list (cramped expert UI).
* Section headers were default body weight (weak hierarchy).

Excellent pieces kept: searchable list, groups, Sample glass CTA, empty/error
states, connect-on-activate, context menus, URL import.

## Design decision

One coherent direction: **scannable connection index**.

| Element | Treatment |
|---|---|
| Engine | Monospaced badge `PG` / `CH` / `RD` / `SQ` |
| Live state | `WORD` monospaced + optional detail (`HEALTHY · 12 ms`) |
| Active window | `ACTIVE` word + subtle list row accent (not color alone) |
| Production | Existing `HALO PRODUCTION` text |
| Plaintext secret | `PLAINTEXT SECRET` word (not orange-only) |
| Connected layout | `VSplitView` profiles ↔ catalog (resizable) |
| Actions chrome | Glass cluster unchanged (New / Sample / Group / URL) |

Spatial: list remains primary; catalog is a **peer pane** when connected, not a
footer. Selection still activates Connect (product: click to connect); menu
holds Edit/Test/etc.

## Research principles

* Finder / Things / Xcode navigator: scannable leading codes + dense rows.
* Raycast / Linear: short status words over prose.
* HIG: non-color semantics; sidebar list style.
* Competitors (workflow existence): connection list is the daily entry point—
  density and live state matter more than decoration.

## Implementation

* `WorkbenchTypes.swift` — `ProfileEngineBadge`, `ProfileLiveStatePresentation`
* `TableRockApp.swift` — `ConnectionsProfileList`, `ConnectionsCatalogPane`,
  redesigned `ProfileRow`, `VSplitView` when session open
* Tests: `ProfilePresentationTests.swift`

## Validation

```text
cd native && swift build --target TableRockFeature
cd native && swift build --target TableRockApp
```

## Remaining weaknesses

* Click-to-connect still couples selection and open (no idle selection).
* No multi-select bulk actions.
* Group drag-and-drop ordering still limited.

## Next improvement for Connections

**Idle selection + Return-to-connect** (Finder-like): select without opening,
⌘O/Return connects; preserves menu density and reduces accidental connects.
