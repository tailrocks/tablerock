# Value Inspector craft pass

Date: 2026-08-09

## Path

```text
AppKit grid selectCell(row, column)
  → BridgeModel.selectedCellSnapshot
  → WorkbenchColumn + WorkbenchCell (UniFFI-shaped)
  → GridCellPresentation.project (kind glyph / a11y)
  → NativeValueInspector (SwiftUI opaque plane)
  → StructuredValueTree.decode (bounded JSON tree)
  → ValueInspectorProjection.hexLinear / hexDump (pure)
```

Rust still owns typed page decode, truncation, and raw bytes. Swift never invents
database semantics. Continuum peer plane still replaces the inspector when open.

## Research (principles only)

| Reference class | Principle applied |
|---|---|
| Xcode / Pixelmator inspectors | Kind-first chrome; facts dense; content opaque |
| Continuum plane (own product) | Uppercase section labels, monospaced identity, fixed header |
| TUI inspector (own product) | Text + hex dump + tree; truncation/stale as text |
| Professional hex tools | Offset · hex · ASCII gutter, not a single endless line |
| HIG / Tahoe | No glass on content; borderless copy actions; text selection |
| Awards (clarity / progressive disclosure) | Hex progressive when large; tree fail-closed |

Competitors (TablePlus/DataGrip class) establish that a trailing cell inspector
exists. No layouts, copy, chrome, or key bindings were copied.

## Before

| Area | Problem |
|---|---|
| Hierarchy | Generic “Value Inspector” title + form `LabeledContent` rows |
| Density | GroupBox chrome on every projection; expert space wasted |
| Hex | Single-line space hex only; no offset/ASCII dump |
| Kind | Kind was a labeled row, not the primary identity |
| Empty / NULL | No special surface for NULL or empty text |
| Structured fail | Silent omission when tree decode failed |
| Copy | No in-inspector copy for text/hex (only result copy menu) |
| Continuum | Already excellent: peer plane replaces inspector |

Excellent kept: selection-driven open/hide, min width 180, opaque
`textBackgroundColor`, a11y ids `value.inspector` / `.tree`, bounds 64 KiB /
1024 nodes / 64 depth, fixture audit, Continuum replacement.

## Design decision

One coherent direction: **typed value instrument**.

| State | Behavior |
|---|---|
| Resting (no cell) | Inspector absent; grid uses full split |
| Selected | Kind glyph + KIND · column · R/C header; dense metadata strip |
| NULL | ∅ + NULL word; non-color |
| Empty text | · + Empty text |
| Text / number / temporal | Monospaced TEXT primary |
| Binary | Hex dump primary (offset · bytes · ASCII) |
| Structured | JSON tree primary when decode ok; fail-closed caption otherwise |
| Truncated | Scissors label + metadata “truncated from N B” |
| Hex secondary | Always-visible linear hex; multi-line dump when multi-line or large |
| Loading / error | N/A for local projection; page failure clears selection snapshot |
| Disabled copy | Copy Hex when zero bytes; Copy Text when null empty |

Fixed: workbench split, continuum replacement, a11y identifiers.
Moves: nothing permanent; inspector is progressive disclosure of the selection.
Permanent: kind + column + location. Progressive: hex dump detail, tree.

Why better: hierarchy matches Continuum and expert TUI; content dominates;
kind is scannable in one glance; hex is readable for binary work.

## Implementation

| File | Change |
|---|---|
| `WorkbenchTypes.swift` | `GridCellPresentation.kindGlyph`; `ValueInspectorProjection` pure helpers |
| `TableRockApp.swift` | Rewrite `NativeValueInspector` |
| `ValueInspectorProjectionTests.swift` | Pure tests (Swift Testing) |
| `design-system.md` | Inspector row in screen audit |

## Validation

```text
cd native && swift build --target TableRockFeature
cd native && swift build --target TableRockApp
```

Both passed on this host (macOS Command Line Tools SDK).

`swift test --filter valueInspector` requires full Xcode/`XCTest` for the
existing XCTest suites in the same target; host has only CLT. Pure projection
tests are present for Xcode CI.

UITests still target `value.inspector` and `value.inspector.tree` (unchanged
identifiers). Fixture audit still requires visible text + linear hex + tree model.

Manual visual checklist (operator): light/dark, narrow split ≥180 pt, NULL /
text / binary / structured cells, Continuum open replaces inspector, Reduce
Transparency (opaque plane remains).

## Remaining weaknesses

* No staged/original compare on native (TUI has it; needs draft projection).
* Hex window paging (Hex+/Hex−) not yet on native for multi-MiB blobs.
* Tree is flat indented list, not outline expand/collapse per node.
* Cannot run XCUITest on CLT-only agents.

## Next improvement for Value Inspector

**Staged vs original compare block** when the selected cell has a pending draft
(parity with TUI evidence 374/396), still driven by Rust staging truth.
