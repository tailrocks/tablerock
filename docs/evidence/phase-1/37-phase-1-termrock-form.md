# Phase 1 TermRock Form Evidence

## Checkpoint

TermRock's neutral `Form` landed on `main` in implementation commit
[`176c00f5`](https://github.com/tailrocks/termrock/commit/176c00f5c76d15ddebbd3ed821fa452b0eb62673).
Compatibility evidence followed in
[`e5bd94e2`](https://github.com/tailrocks/termrock/commit/e5bd94e2b6f803a8da7a25ff167cc44f51fe6c0b),
which is TableRock's new exact dependency revision.

The public widget borrows caller-owned sections and stable-ID field
projections. It renders caller wording and values with required, disabled,
help, and validation-error facts. One/two-column reflow preserves semantic
focus order. Keyboard, mouse, wheel, and scrollbar actions produce semantic
focus/activation or bounded scroll while the caller retains editing,
validation, submission, and lifecycle policy.

Painted fields expose clipped union geometry plus optional label, value, and
supporting-text rectangles. TableRock can therefore compose real input widgets
without deriving layout from private details. Required and disabled states use
reserved neutral `*` and `⊘` non-color markers; caller text cannot overwrite
them or split a wide grapheme.

## Evidence

- Six public-API Form tests cover multi-section traversal, disabled skipping,
  wrap/back-wrap, Home/End, activation, focus gating, reflow stability, hover,
  click, partial clipping geometry, scrolling/track input, validation/help,
  paintable empty/tiny rectangles, non-color state, and wide graphemes.
- Interactive lookbook Form story and deterministic SVG preview are published;
  hover clearing outside the preview has a regression test.
- Full TermRock workspace: 207 tests pass; formatting, Clippy, all-feature and
  no-feature/example builds, rustdoc, packaging, semver (196/196), dependency
  policy, secret scan, generated API, deterministic previews, and docs pass.
- Independent standards and specification reviews returned no findings after
  four structural review fixes.
- Jackin `27c450e9` compiles across all workspace targets against its earlier
  compatible TermRock pin. Its aggregate CI has unrelated baseline failures:
  three Rust 1.97 `unused_self` lints and two of 5,372 tests; these do not touch
  TermRock or Form and are recorded upstream in `compatibility.toml`.

This checkpoint does not claim T1 `SplitPane`, TableRock connection forms, or
the executable shell.

External concept: generic structured settings forms only  
Public source: <https://github.com/tailrocks/termrock/tree/e5bd94e2b6f803a8da7a25ff167cc44f51fe6c0b>  
TableRock requirement: Roadmap Phase 1 / delivery-plan TermRock T1  
Implementation source: TableRock requirements, TermRock public conventions, and independent tests  
Copied code/assets/text: none
