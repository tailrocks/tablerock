# TermRock 0.9 Canonical Module Migration

Status: accepted and integrated on 2026-07-16.

TableRock advances its exact TermRock `main` pin from `ff263f2` to
`37d1fda4c334e7c3c9210c3301ff371a3aa8d2ce` (`0.9.0`). TermRock's sequential
`MIGRATING.md` links the separate
`0010-v0.9.0-canonical-module-homes.md` guide after `0009`.

## Old to new

TermRock removes duplicate crate-root and `geometry`/`theme` paths. Text,
layout, scrolling, widget geometry, style, and OSC pointer concepts now each
have one canonical module home.

TableRock already imports widget contracts through `widgets`, hit regions
through `interaction`, runtime contracts through `runtime`, and keeps the
documented root `Theme` entry point. It imports none of the removed duplicate
paths or pointer helpers. Migration therefore updates only the exact pin and
lockfile; no alias, shim, or competing module vocabulary remains.

## Verification

- Source inspection finds no removed root/geometry/theme/pointer path.
- Root TEA render, interaction, input, and PTY lifecycle fixtures pass at the
  exact new revision.
- Workspace tests, Clippy, and rustdoc pass.

External concepts: one canonical public module home per concept
Public source: <https://github.com/tailrocks/termrock/tree/37d1fda4c334e7c3c9210c3301ff371a3aa8d2ce>
Implementation source: TableRock dependency pin only
Copied code/assets/text: none
