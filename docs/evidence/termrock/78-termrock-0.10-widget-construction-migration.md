# TermRock 0.10 Widget Construction Migration

Status: accepted and integrated on 2026-07-16.

TableRock advances its exact TermRock `main` pin from `ccf0646` to
`a3f39b71dd8107041b2941e8e6366ee9037dc7aa` (`0.10.0`). TermRock's sequential
`MIGRATING.md` links the separate
`0012-v0.10.0-widget-construction-and-growth.md` guide after `0011`.

## Old to new

Renderable widgets now have private fields, one required-argument constructor,
optional builders, owned and borrowed render implementations, and
growth-oriented non-exhaustive enums.

TableRock replaces all remaining `Tabs`, `ActionBar`, `HintBar`, `StatusBar`,
and `Panel` field literals with canonical constructors/builders. Required data
and the one root-owned theme are passed to `new`; optional gap/separator/alpha/
title/emphasis use builders. No local constructor wrapper, field-mutation shim,
or old literal path remains.

TableRock does not yet exhaustively match the newly non-exhaustive component
enums. Future matches must include a fallback and keep domain decisions in the
root reducer.

## Verification

- Source search finds no literal construction of migrated widgets.
- Narrow/medium/wide renders and interaction geometry remain byte-for-byte
  covered by existing fixtures.
- Root TEA, input, and PTY lifecycle tests pass at the exact revision.
- Workspace tests, Clippy, and rustdoc pass.

External concepts: constructor/builder widget APIs and forward-compatible public enums
Public source: <https://github.com/tailrocks/termrock/tree/a3f39b71dd8107041b2941e8e6366ee9037dc7aa>
Implementation source: TableRock root view construction migration
Copied code/assets/text: none
