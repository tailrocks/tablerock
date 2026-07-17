# TermRock 0.10 Lookbook Output Hardening Update

## Upstream change

TableRock now pins exact TermRock `main` revision
`534e3c2db8de18da84f0d5a1211c136d0cb2d898`. Relative to the prior pin
`bbc6c980389e49f4306a8e65a71ce11f280147e7`, this revision hardens generated
lookbook JSON/SVG output, removes a stale generated manifest, and updates
lookbook-local documentation. It changes no `termrock` library source or public
API and adds no migration.

## TableRock impact

No source migration is required. TableRock does not consume generated lookbook
artifacts at runtime. The changes improve upstream component evidence without
altering TableRock's pinned API or TUI behavior.

## Verification

- Exact Git revision resolves in the lockfile.
- Workspace tests and lint pass against the revision.
- No TableRock behavior or support claim changes.

External concepts: deterministic generated documentation artifacts
Public sources: <https://github.com/tailrocks/termrock/commit/534e3c2db8de18da84f0d5a1211c136d0cb2d898>
Implementation source: TermRock upstream lookbook changes and TableRock-owned compatibility inspection
Copied code/assets/text: none
