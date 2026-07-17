# TermRock 0.10 Plan Reconciliation Update

## Upstream change

TableRock now pins exact TermRock `main` revision
`ade6aa0b394b0afac1da0237d83390b9d5441668`. Relative to the prior pin
`20318221c792ee0a0d0145967321adaee57875ae`, this revision changes only TermRock
planning documents. It reconciles widget plans and adds plans for neutral input
contract completion, Rustdoc placeholder removal, and MultiSelect contract
alignment. It changes no crate source or public API and adds no migration.

## TableRock impact

No source migration is required. TableRock retains its neutral input boundary
and current TermRock imports until the planned upstream changes land. Each
future incompatible implementation will be adopted without compatibility shims
using its sequential TermRock migration document.

## Verification

- Exact Git revision resolves in the lockfile.
- Workspace tests, lint, and documentation pass against the pin.
- No TableRock behavior or support claim changes.

External concepts: none
Public sources: <https://github.com/tailrocks/termrock/commit/ade6aa0b394b0afac1da0237d83390b9d5441668>
Implementation source: TermRock upstream planning documents and TableRock-owned compatibility inspection
Copied code/assets/text: none
