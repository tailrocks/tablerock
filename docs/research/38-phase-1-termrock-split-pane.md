# Phase 1 TermRock SplitPane Evidence

## Checkpoint

TermRock's neutral `SplitPane` landed on `main` in implementation commit
[`b6cd8c41`](https://github.com/tailrocks/termrock/commit/b6cd8c4124a95267a9ba5c25e41be917192f4654).
Compatibility evidence followed in
[`8cb3c88d`](https://github.com/tailrocks/termrock/commit/8cb3c88d118b2cbed10eef9d7cdbf0c0adbbbfde),
which is TableRock's new exact dependency revision.

The public primitive maps horizontal/vertical direction, an integer remembered
ratio, and caller minimums into bounded first-pane, divider, and second-pane
rectangles. Impossible minimums degrade proportionally with overflow-safe
integer arithmetic. Collapse/expand preserves the ratio and remains explicitly
caller mapped.

Only rendering publishes private direction-tagged pointer geometry. Pure
layout computation cannot manufacture a hit target; stale direction and empty
repaint geometry are rejected. Keyboard resizing, divider focus/hover,
painted-divider down/drag/up, collapse glyphs, and tiny cross-axis rectangles
have symmetric horizontal/vertical evidence. Pane content, persistence, focus
routing, and collapse policy remain caller owned.

## Evidence

- Nine public-API SplitPane tests cover minimums, ratios, proportional tiny
  degradation, maximum integer bounds, both axes, zero dimensions, keyboard
  focus/gating, pointer lifecycle, off-divider rejection, collapse/expand,
  remembered ratio, non-color glyphs, computed-versus-painted authority,
  direction mismatch, and stale-hit invalidation.
- Interactive lookbook story and deterministic SVG preview are published; its
  real pointer drag path has a regression test.
- Full TermRock workspace: 217 tests pass; formatting, Clippy, all-feature and
  no-feature/example builds, rustdoc, packaging, semver (196/196), dependency
  policy, secret scan, generated API, deterministic previews, and docs pass.
- Jackin `27c450e9` compiles across all workspace targets with a temporary exact
  pin to `17974590`; its manifest and lockfile were restored clean afterward.
  TermRock records the command and result in the pinned compatibility evidence.
- Independent standards and specification reviews returned no findings after
  rendered-authority, proportional-minimum, vertical-evidence, and
  caller-collapse-policy corrections.

T1 `Tree`, `Form`, and `SplitPane` prerequisites are now published. This
checkpoint does not claim the TableRock executable shell or Phase 1 exit.

External concept: generic resizable pane layout only  
Public source: <https://github.com/tailrocks/termrock/tree/8cb3c88d118b2cbed10eef9d7cdbf0c0adbbbfde>  
TableRock requirement: Roadmap Phase 1 / delivery-plan TermRock T1  
Implementation source: TableRock requirements, TermRock public conventions, and independent tests  
Copied code/assets/text: none
