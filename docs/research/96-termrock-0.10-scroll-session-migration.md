# TermRock 0.10 Scroll and Session Migration

## Upstream change

TableRock now pins exact TermRock `main` revision
`4c3adace5f440e2a5ca737d12bae096ee71b4df9`. Relative to
`0089bd7bcd99086be1e7c7bf4753a733ef0bd935`, this revision publishes two
sequential breaking migrations and establishes a versioned release flow.

Migration `0014` removes duplicate dialog, scroll-render, and hover surfaces in
favor of canonical `scroll`, `widgets`, Ratatui layout, and `HoverState`
contracts. TableRock used none of the removed APIs, so it carries no shim and
requires no source replacement.

Migration `0015` makes alternate screen, cursor visibility, line wrapping,
mouse capture, bracketed paste, and raw mode independent `SessionOptions`.
TableRock's production sessions use `SessionOptions::default()` and inherit the
intended full-screen defaults. Its no-live-terminal test uses a default-based
literal, exercising the new cursor and line-wrap cleanup while avoiding live
terminal output and remaining open to future option growth.

## Verification

- Every migration after the prior pin was read in numeric order.
- All published TermRock features remain enabled.
- TableRock contains no removed scroll, dialog, hover, or modal API usage.
- Workspace tests, lint, documentation, and dependency policy pass on the new
  exact revision.

External concepts: canonical support surfaces, independent terminal modes, immutable release tags
Public sources: <https://github.com/tailrocks/termrock/blob/4c3adace5f440e2a5ca737d12bae096ee71b4df9/migrations/0014-v0.10.0-scroll-and-hover-unification.md>, <https://github.com/tailrocks/termrock/blob/4c3adace5f440e2a5ca737d12bae096ee71b4df9/migrations/0015-v0.10.0-independent-session-options.md>
Implementation source: TermRock migration documents and TableRock-owned compatibility audit
Copied code/assets/text: none
