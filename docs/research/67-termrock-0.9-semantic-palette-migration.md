# TermRock 0.9 Semantic Palette Migration

Status: accepted and integrated on 2026-07-16.

TableRock advances its exact TermRock `main` pin from `4bbb6de` to
`4b980aa98bf135eac197fb32cdc58699df2eb62c` (`0.9.0`). TermRock's sequential
`MIGRATING.md` links the separate
`0008-v0.9.0-semantic-theme-palette.md` before/after guide after migration
`0007`.

## Old to new

TermRock removes donor-product palette constants and raw preset internals from
the crate root. Reusable components continue to consume semantic `Theme` and
`Role` values. A consumer needing a raw reusable preset value imports the
documented value from `termrock::style`; product-specific colors remain owned
by that consumer.

TableRock already owns one complete root theme and never imported the removed
raw or donor-branded constants. Therefore the forward migration is an exact pin
and lockfile update with no compatibility alias, duplicate palette, or source
change. All existing widgets continue receiving the root-owned immutable theme.

## Verification

- TableRock contains no import of a removed palette symbol.
- Existing narrow/medium/wide render and interaction fixtures pass at the new
  exact revision.
- Workspace build, tests, Clippy, and rustdoc pass.

External concepts: semantic theme roles and consumer-owned product palettes
Public source: <https://github.com/tailrocks/termrock/tree/4b980aa98bf135eac197fb32cdc58699df2eb62c>
Implementation source: TableRock dependency pin only
Copied code/assets/text: none
