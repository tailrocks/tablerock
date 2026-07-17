# TermRock 0.10 Visible Scroll Update

Status: accepted and integrated on 2026-07-16.

TableRock advances its exact TermRock `main` pin from `1130d8c` to
`5c4758b5167241ffef5f501f72bb76bc01e77aba` (`0.10.0`). The intervening changes
render only visible scroll content, flatten list checkbox rendering, add
viewport hot-path evidence, and document the panel focus hierarchy.

## Compatibility decision

The public contracts TableRock consumes are unchanged. TermRock correctly adds
no breaking migration after sequential migration `0012`. TableRock reviewed the
complete diff and retains the canonical widget constructors, one root focus
owner, render-authorized hit geometry, and responsive shell.

The visible-slice implementation aligns with TableRock's future large catalog,
grid, history, and editor requirements. TableRock adds no local scroll fork,
compatibility shim, or duplicated viewport implementation.

## Verification

- Narrow, medium, and wide shell fixtures pass at the exact revision.
- Input geometry, focus, bounded ingress, and PTY restoration tests pass.
- Workspace tests, Clippy with warnings denied, and rustdoc pass.

External concepts: viewport-bounded rendering and explicit focus hierarchy
Public source: <https://github.com/tailrocks/termrock/tree/5c4758b5167241ffef5f501f72bb76bc01e77aba>
Implementation source: TermRock-owned rendering and documentation; unchanged TableRock consumers
Copied code/assets/text: none
