# TermRock 0.9 Slate Preset Update

Status: accepted and integrated on 2026-07-16.

TableRock advances its exact TermRock `main` pin from `4b980aa` to
`a002902d0669cf04fe07d9ffcecbd97f6e58df19` (`0.9.0`). The published change is
additive: `Theme::slate()` provides a complete alternate-palette proof and the
lookbook verifies both presets. No existing public contract changed, so
TermRock's sequential migration index correctly remains at `0008`.

## Consumer decision

TableRock keeps its approved default phosphor theme from
`11-terminal-experience.md`. It does not switch product appearance merely
because a library proof preset exists. The new preset strengthens confidence
that every reusable widget follows semantic roles and that a later explicitly
approved TableRock palette can replace the default without widget-local edits.

No compatibility facade, copied palette, duplicate theme owner, or source
adaptation is introduced. The root model remains the sole theme owner.

## Verification

- The exact new revision builds through both TableRock TUI consumers.
- Existing narrow/medium/wide render and interaction fixtures pass unchanged.
- Workspace tests, Clippy, and rustdoc pass.

External concepts: complete alternate semantic-theme preset
Public source: <https://github.com/tailrocks/termrock/tree/a002902d0669cf04fe07d9ffcecbd97f6e58df19>
Implementation source: TableRock dependency pin only
Copied code/assets/text: none
