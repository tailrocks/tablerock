# TermRock Migration 0019 Adoption

## Published boundary inspected

TableRock refreshed its exact TermRock `main` pin from
`ac54f9194d4a58b08bd1d824266f4c5ba1357317` to
`151aafc0abc46057fdb65532b5bbe551ea3fe369`. The published change is migration
0019's breaking completion of bounded `LogPane` scrollback.

## Old to new

`LogPaneState::new()` now retains at most 10,000 lines. Unbounded retention is
an explicit `.unbounded()` opt-in. Neutral wheel input uses `scroll_by`, follow
state uses `follow`/`is_following`, Home reaches the oldest retained window, and
ANSI ingestion uses `ansi_text::line_from_ansi`.

TableRock does not yet consume `LogPane`; no source adaptation or compatibility
layer is needed. Future diagnostic/output surfaces must use the bounded default,
route neutral TermRock input, and account for oldest-line eviction. TableRock
must not opt into unbounded history because its architecture requires finite
resident state.

## Verification

The TableRock workspace builds, tests, lints, and documents against the exact
new revision. Dependency and policy gates remain part of this checkpoint.

Primary source: TermRock's published
`migrations/0019-v0.11.0-bounded-log-pane-scrollback.md` at revision `151aafc`.
