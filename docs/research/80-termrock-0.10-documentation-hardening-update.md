# TermRock 0.10 Documentation Hardening Update

Status: accepted and integrated on 2026-07-16.

TableRock advances its exact TermRock `main` pin from `a3f39b7` to
`1130d8c1d16489b2f510d93bac07f5370ba2cbc7` (`0.10.0`). The intervening commits
deny undocumented public items, document public modules and widgets, mark
private integration-test crates intentionally private, and replace fragmented
example stubs with one runnable showcase.

## Compatibility decision

This update adds documentation and example coverage without changing the
public contracts TableRock consumes. Therefore TermRock correctly adds no new
breaking migration after sequential migration `0012`. TableRock still reviewed
the complete diff and rebuilt every TUI, interaction, and PTY consumer at the
exact revision.

No compatibility shim, duplicate example path, or local TermRock copy is added.
Future incompatible changes still require the next separate sequential
TermRock migration file and consumer migration evidence.

## Verification

- TableRock uses only documented canonical constructors and module paths.
- Workspace tests, Clippy with warnings denied, rustdoc, TUI renders,
  interaction geometry, and PTY lifecycle tests pass at the exact revision.

External concepts: complete public API documentation and runnable examples
Public source: <https://github.com/tailrocks/termrock/tree/1130d8c1d16489b2f510d93bac07f5370ba2cbc7>
Implementation source: TermRock-owned documentation; unchanged TableRock consumers
Copied code/assets/text: none
