# TermRock 0.9 Styled Tab Glyph Migration

## Checkpoint

TableRock advances its exact TermRock pin from
`da54a033f368ed0888af90ae43d19bcb96fb8581` (`0.8.0`) to current `main`
`c51e11cf011da3eba836fa368993bf37c14834ba` (`0.9.0`). Styled glyph migration
`0003` entered at `bb8ff316`; the two following commits document TermRock's
modern-first API policy and deep-audit plan without changing the consumer API.
All commits are signed off, synchronized to TermRock `origin/main`, and retain Rust 1.95,
Crossterm 0.29, Ratatui Core 0.1.2, and the established renderer tuple.

## Sequential migration

TermRock's `MIGRATING.md` indexes one new sequential migration:
`0003-v0.9.0-styled-tab-glyphs.md`. `Tab::glyph` changes from `Option<&str>` to
`Option<ratatui_core::text::Span>`, making semantic glyph style caller-owned
while TermRock retains tab geometry, clipping, focus, fill, and hit regions.

TableRock currently supplies `glyph: None`, so the compiler-proven consumer edit
is only the exact dependency/version advance. Future glyphs must use
`Span::raw` or `Span::styled`; no compatibility shim, old glyph type, buffer
patch, or coordinate reconstruction is retained.

## Concurrent-work handling

TermRock's worktree contains another agent's uncommitted `AGENTS.md` and `plans/`
changes. TableRock inspected the published `main` commit and migration files,
did not modify or remove those concurrent files, and pins the synchronized
remote commit rather than relying on uncommitted state.

## Verification record

- Exact manifest and lockfile source/version/revision: pass.
- TableRock tab call-site audit: only `glyph: None`; no API conflict.
- `cargo test --workspace --locked`: 84 passed, 3 ignored.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: pass.
- Workspace format, rustdoc, `cargo deny`, `gitleaks`, English-script, and
  complete-diff gates: pass. The already-allowed `hashbrown` duplicate is
  unchanged.

External concepts: styled glyph span owned by the consumer
Public source: <https://github.com/tailrocks/termrock/tree/c51e11cf011da3eba836fa368993bf37c14834ba>
TableRock requirements: research 13, 20, 30, 32, and 33
Implementation source: TermRock public API and sequential migration 0003
Copied code/assets/text: none
