# Data Grid: typed cell presentation pass

Date: 2026-08-09

## Path

```text
Rust ResultPage / OwnedValue
  → UniFFI page bytes / WorkbenchCell { display, kind, truncation, bytes }
  → GridCellPresentation.project (TableRockFeature, pure)
  → CatalogGrid NSTableView cell paint (AppKit)
```

## Before

Native `CatalogGrid` painted raw `rows[r][c]` strings only:

* NULL / empty / binary / truncated were not distinguished by glyph+text;
* numeric columns left-aligned like free text;
* accessibility value was the raw string without kind;
* empty results showed a blank table chrome with no empty state;
* selection had no status fact line.

Product `data-grid.md` already required typed distinctions never by color alone;
TUI `CellDistinction` already had glyphs. Native lag was a **presentation gap**,
not missing Rust truth.

## Design decision

One coherent direction: **typed dense grid**.

| State | Presentation |
|---|---|
| NULL | `∅` + a11y “NULL” + secondary tint *with* glyph |
| Empty text | `·` + a11y “Empty text” |
| Binary | `⟨b N⟩` |
| Structured empty | `{}` |
| Truncated | `…` prefix + a11y “Truncated …” |
| Invalid / unknown | `!` / `?` glyphs |
| Numbers | monospaced digits + right alignment |
| Resting | alternating rows, line border (not heavy bezel), opaque `textBackgroundColor` |
| Empty result | `ContentUnavailableView` |
| Selection | status line `column · kind · facts · R/C` |

Glass remains off content. Density preserved (small row size).

## Research (principles only)

* Professional data tables (finance/ops): monospaced digits, numeric alignment.
* TUI TableRock `CellDistinction` glyphs (same product, shared language).
* Apple HIG: non-color semantics; content not glass; accessibility labels include role and value.
* Postico/TablePlus class: dense grids (workflow existence); no layout copy.

## Implementation

* `native/Sources/TableRockFeature/WorkbenchTypes.swift` — `GridCellPresentation`
* `native/Sources/TableRockApp/TableRockApp.swift` — `CatalogGrid` paint + empty/selection chrome
* `native/Tests/TableRockFeatureTests/GridCellPresentationTests.swift`

## Validation

```text
cd native && swift build --target TableRockApp
cd native && swift build --target TableRockFeature
# XCTest may be unavailable under CLT-only hosts; pure presentation unit tests are still source of truth.
```

## Remaining weaknesses

* No live Instruments screenshot in headless agent environment.
* Staged-mutation row markers not yet projected on native grid paint.
* Column width persistence / fit-to-content still open (prior evidence 617).

## Next improvement for Data Grid

Project **Change Ledger draft markers** (insert / modify / delete glyphs) into
native cell paint when staged facts exist—parity with TUI VirtualGrid.
