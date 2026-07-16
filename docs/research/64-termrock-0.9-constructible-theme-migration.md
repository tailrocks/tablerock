# TermRock 0.9 Constructible Theme Migration

Status: accepted and integrated on 2026-07-16.

TableRock advances its exact TermRock `main` pin from `7d8569d` to
`4bbb6de377574098c133e32a2f455bebac783c9b` (`0.9.0`). TermRock's sequential
`MIGRATING.md` links the separate
`0007-v0.9.0-constructible-theme.md` before/after guide.

## Old to new

Tabs, actions, hints, status bars, diffs, and viewport chrome no longer own
disconnected raw base styles. Each consumes a borrowed complete `Theme` and
selects semantic `Role` values internally. Consumers construct or override the
palette through `Theme::from_fn` and `Theme::with_role`.

TableRock already kept one root-owned theme and passed it to panels. This
migration passes that same immutable theme to `Tabs`, `ActionBar`, `HintBar`,
and `StatusBar`; the removed `StatusBar.style` field disappears. No duplicate
theme, compatibility facade, widget-specific palette, or presentation state is
added.

## Verification

- Existing narrow/medium/wide render and interaction fixtures exercise the
  semantic-theme widgets at the exact new revision.
- The root model remains the sole theme owner.
- Workspace build, tests, Clippy, and rustdoc pass.

External concepts: semantic theme roles and caller-owned palette construction
Public source: <https://github.com/tailrocks/termrock/tree/4bbb6de377574098c133e32a2f455bebac783c9b>
Implementation source: TableRock root theme adaptation only
Copied code/assets/text: none
