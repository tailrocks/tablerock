# TermRock Migration 0020 Adoption

## Published boundary inspected

TableRock refreshed its exact TermRock `main` pin from
`151aafc0abc46057fdb65532b5bbe551ea3fe369` to
`f802fcc48c4361ea477c5021b52a121f180d4b4d`. The published range hardens bounded
log history, adds migration 0020's explicit oldest navigation, and completes
the semantic progress rendering contract.

## Old to new

`LogPaneState::scroll_to_oldest()` is now the canonical semantic transition for
buttons, restored state, and commands. It works before first render by retaining
pending intent until viewport geometry exists. Keyboard adapters may still
forward Home; non-keyboard controls must not synthesize a key event.

TableRock does not yet render `LogPane` or TermRock's progress widget, so no
consumer source changed. Future surfaces must use bounded history,
`scroll_to_oldest` for semantic navigation, and the completed TermRock progress
renderer instead of building parallel product-local primitives.

## Verification

The complete TableRock workspace builds, tests, lints, and documents against
the exact new revision. Dependency and source-policy gates remain part of the
checkpoint.

Primary source: TermRock's published
`migrations/0020-v0.11.0-log-pane-oldest-navigation.md` and public API at
revision `f802fcc`.
