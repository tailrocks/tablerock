# TermRock 0.11 Migration Evidence

## Adopted sequence

TableRock moved directly from TermRock 0.10 to 0.11 at exact `main` revision
`50d67f17e1a027a954c91ef68275afb438eed300`. The upstream migration index was
read in order:

1. `0016-v0.11.0-text-strong-and-viewport-emphasis.md`
2. `0017-v0.11.0-scrollable-block-helpers.md`

## Consumer changes

Migration 0016 separates ordinary `Role::Text` from semantic
`Role::TextStrong` and gives `Viewport` first-class `PanelEmphasis`. Migration
0017 establishes `termrock::scroll` as the canonical home for dialog scrolling,
scroll hints, line-offset rendering, and scrollable-block helpers.

TableRock's current shell uses none of the replaced/manual surfaces: it has no
manual bold `Role::Text`, viewport theme surgery, `layout` scroll imports,
`DialogBodyScroll`, or product-local scrollable-block renderer. Therefore the
correct migration is immediate version/revision adoption with no compatibility
alias or dead transitional path. New work must use the 0.11 semantic roles,
viewport emphasis, and `termrock::scroll` APIs directly.

## Verification

The complete TableRock workspace builds, tests, lints, and documents with both
published TermRock features (`crossterm`, `serde`) enabled by the single
workspace dependency declaration. Source search proves no replaced call site.

External concepts: TermRock public API migration
Public sources: <https://github.com/tailrocks/termrock/blob/main/MIGRATING.md>, <https://github.com/tailrocks/termrock/tree/main/migrations>
Implementation source: TableRock dependency pin and consumer audit
Copied code/assets/text: none
