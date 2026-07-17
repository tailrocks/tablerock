# TermRock 0.10 Picker Spike Update

## Upstream change

TableRock now pins exact TermRock `main` revision
`3a80ef0c4749bd98643bcb42869293bef2cb4733`. Relative to the prior pin
`20ef50684f5e0ad9871f667ed425666b5e202a40`, this revision adds a
lookbook-local caller-filtered picker composition spike and design record. It
changes no `termrock` library source or public API and adds no migration.

This historical lookbook-only status was superseded when Picker graduated into
the public API; see
[`134-termrock-picker-graduation-update.md`](134-termrock-picker-graduation-update.md).

The chosen direction composes existing input/list/panel behavior while leaving
filtering and product policy with the caller. That is compatible with
TableRock's future quick-switch and completion projections; TableRock will use
the published neutral composition after it graduates, never import lookbook
internals.

## Verification

- Exact Git revision resolves in the lockfile.
- Workspace tests and lint pass against the revision.
- No direct source migration is required.

External concepts: caller-filtered picker composition
Public sources: <https://github.com/tailrocks/termrock/commit/3a80ef0c4749bd98643bcb42869293bef2cb4733>
Implementation source: TermRock upstream lookbook spike and TableRock-owned compatibility inspection
Copied code/assets/text: none
