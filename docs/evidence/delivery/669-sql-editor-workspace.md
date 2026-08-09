# SQL Editor workspace pass

Date: 2026-08-09

## Path

```text
Tab statementText (presentation model)
  → SqlTextEditor NSTextView (AppKit, IME-safe binding)
  → Run → UniFFI execute → result page → ResultGridWithInspector
```

Rust still owns parse/execute/diagnostics; Swift owns editor chrome and layout.

## Before

* Editor capped at ~120 pt height; not a real resizable split vs results.
* Action chrome mixed “Apply probe” equally with Run; status below chrome.
* No line numbers (spec: line numbers + current statement mark).
* Heavy bezel border; no horizontal scroller emphasis.
* No caret metrics (L/C/lines) or RUNNING word in editor chrome.
* Spec order editor → run bar → results not honored.

Excellent kept: monospaced NSTextView, IME non-clobber, undo, a11y
`query.editor`, Find/Replace sheet, parameterized run sheet, ⌘R.

## Design decision

One coherent direction: **statement workspace**.

| Region | Behavior |
|---|---|
| Header | SQL · file · caret metrics · HALO PRODUCTION if needed |
| Editor | NSTextView + line-number ruler; line border; opaque |
| Split | `VSplitView` editor above results pane |
| Action strip | Between editor and results: Run / Cancel / Find / … + status |
| Running | Soft dim text color + RUNNING in metrics (still editable) |
| Empty results | ContentUnavailable under the strip |

Fixed: workbench shell. Moves: split proportions. Progressive: Find sheet,
parameters sheet, explain.

## Research principles

* iA Writer / Xcode: monospaced focus, metrics chrome, content not glass.
* Product `sql-editor.md` layout diagram.
* Professional SQL clients: editor/results split, not fixed stub height.
* HIG: focus ring, native find bar on NSTextView, non-color RUNNING/ERROR.

## Implementation

* `WorkbenchTypes.swift` — `SqlEditorMetrics`
* `TableRockApp.swift` — `QueryWorkbenchView` VSplitView + chrome;
  `SqlTextEditor` ruler, horizontal scroll, `isRunning`; `SqlLineNumberRulerView`
* Tests: `SqlEditorMetricsTests.swift`

## Validation

```text
cd native && swift build --target TableRockFeature
cd native && swift build --target TableRockApp
```

UITests still target `query.editor`, `query.run`, `query.status`.

## Remaining weaknesses

* No syntax highlighting / current-statement underline (needs Rust spans).
* Line gutter is lightweight ruler, not full TextKit 2.
* Split proportions not yet persisted.

## Next improvement for SQL Editor

**Current-statement highlight** from Rust dialect-aware statement ranges
projected as a background mark in the editor (never naive semicolon split).
