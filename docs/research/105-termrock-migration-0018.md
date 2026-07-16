# TermRock Migration 0018 Adoption

## Published boundary inspected

TableRock refreshed its exact TermRock `main` pin from `11b52a35410ef2eccd336ad58cdc114e57594141`
to `ac54f9194d4a58b08bd1d824266f4c5ba1357317`. The published range adds
generated per-component documentation and migration 0018's breaking,
theme-explicit scroll and typed-dialog-input refactor.

The sibling TermRock worktree contained unrelated uncommitted changes from
another agent. TableRock inspected only published history and did not modify,
stage, or overwrite that work.

## Old to new

Migration 0018 removes implicit default themes, raw terminal input parsing in
shared scroll state, duplicate scroll renderers, fixed dialog geometry, and
parallel dialog-border emphasis. Consumers now pass their semantic `Theme`,
adapt backend input before shared state, render bordered content through
`Viewport`, and use typed scrollbar specifications.

TableRock already uses TermRock's neutral input events and does not call any of
the removed dialog or scroll rendering helpers. No compatibility layer was
added. The workspace therefore compiles unchanged against the new forward-only
API, while future scroll UI must start with the migration 0018 replacements.

## Verification

`cargo check --workspace --all-targets --locked` passes at the refreshed pin.
The full workspace test, lint, documentation, dependency, and policy gates run
as part of the same checkpoint.

Primary source: TermRock's published
`migrations/0018-v0.11.0-theme-explicit-scroll.md` at revision `ac54f91`.
