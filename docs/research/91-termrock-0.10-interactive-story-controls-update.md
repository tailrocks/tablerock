# TermRock 0.10 Interactive Story Controls Update

## Upstream change

TableRock now pins exact TermRock `main` revision
`20ef50684f5e0ad9871f667ed425666b5e202a40`. Relative to the prior pin
`534e3c2db8de18da84f0d5a1211c136d0cb2d898`, this revision adds interactive
knobs and story controls to the TermRock lookbook plus its design record. It
changes no `termrock` library source or public API and adds no migration.

## TableRock impact

No source migration is required. The richer lookbook strengthens reusable
component behavior evidence. TableRock continues to consume only the published
library API and does not import lookbook internals.

## Verification

- Exact Git revision resolves in the lockfile.
- Workspace tests and lint pass against the revision.
- No TableRock behavior or support claim changes.

External concepts: interactive component story controls
Public sources: <https://github.com/tailrocks/termrock/commit/20ef50684f5e0ad9871f667ed425666b5e202a40>
Implementation source: TermRock upstream lookbook changes and TableRock-owned compatibility inspection
Copied code/assets/text: none
