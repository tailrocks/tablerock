# TermRock 0.10 Contract-Axis Story Update

## Upstream change

TableRock now pins exact TermRock `main` revision
`0089bd7bcd99086be1e7c7bf4753a733ef0bd935`. Relative to the prior pin
`3a80ef0c4749bd98643bcb42869293bef2cb4733`, this revision expands lookbook
stories and generated documentation evidence across narrow-width, Unicode,
empty, long-content, and interaction contract axes.

The revision changes no `termrock` library source, feature declaration, or
public API. It publishes no migration. TableRock therefore retains its neutral
input integration unchanged while adopting the stronger upstream usage
evidence. The new Unicode stories are test fixtures and do not change
TableRock's English-only product-language rule.

## Verification

- Exact Git revision resolves in the lockfile.
- Both published TermRock features remain enabled.
- Workspace tests, lint, documentation, and dependency policy pass.
- No source migration or compatibility layer is required.

External concepts: contract-axis component stories
Public sources: <https://github.com/tailrocks/termrock/commit/0089bd7bcd99086be1e7c7bf4753a733ef0bd935>
Implementation source: TermRock upstream lookbook evidence and TableRock-owned compatibility inspection
Copied code/assets/text: none
