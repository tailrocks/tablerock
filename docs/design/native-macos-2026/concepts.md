# Native macOS concepts

Selection status: Native Workbench selected and explicitly confirmed by the
operator on 2026-08-12 as the production design. See
[native concept selection](decision-native-workbench.md).

## 1. Native Workbench — confirmed for production

The closest clean-room adaptation of the operator-preferred TablePro public
interface: floating leading catalog, compact context toolbar, document tabs,
dominant opaque grid/editor, bottom mode/status controls, and an optional
trailing inspector. It should feel immediately familiar without reproducing
TablePro branding, copy, assets, or implementation.

Validation focus: density, native hierarchy, uninterrupted working loop, and
whether TableRock-specific safety remains prominent within the familiar frame.

## 2. Query Studio

A narrow mode rail anchors the window. The workspace is an editor-first
vertical split with results and messages docked below; catalog context appears
as a bounded browser instead of a full-height tree.

Validation focus: repeated SQL iteration, editor/result ownership, and compact
navigation tradeoffs.

## 3. Column Observatory

A source column, object column, and detail workspace remain visible together.
This replaces deep tree expansion with spatial drill-down and supports fast
comparison between sibling objects.

Validation focus: orientation across large schemas and the cost of persistent
horizontal structure.

## 4. Grid Canvas

Data or editor content reaches the window edges. Navigation and primary
commands live in detached glass palettes; secondary context collapses into
popover-scale controls.

Validation focus: maximum content area, palette discoverability, and whether
detached chrome remains calm under dense work.

## 5. Change Desk

A persistent trailing ledger makes inserts, updates, deletes, risk, and apply
state visible while browsing or querying. Navigation is compact; review is a
continuation of work rather than a separate destination.

Validation focus: safety, confidence, and whether persistent staged-change
context consumes acceptable space during read-only work.

## Shared evaluation rubric

Each capture is judged on orientation, information density, native macOS fit,
keyboard/focus clarity, resizability, Liquid Glass restraint, accessibility,
workflow continuity, and safety visibility. Concept 1 is the lead candidate by
operator direction; the gate still presents all five without self-approval.
