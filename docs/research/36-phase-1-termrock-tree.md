# Phase 1 TermRock Tree Evidence

## Checkpoint

TermRock's neutral `Tree` landed on `main` in implementation commit
[`8a6f7623`](https://github.com/tailrocks/termrock/commit/8a6f76233a539ed126cda36ee2205656c5a8adc7).
Compatibility evidence followed in
[`cbed106c`](https://github.com/tailrocks/termrock/commit/cbed106c539efbd30dbb9935863e59a7af95bef4),
which is TableRock's new exact dependency revision.

The public widget consumes a caller-flattened borrowed projection with stable
IDs, hierarchy depth, disclosure, enabled, loading, and error facts. It emits
semantic selection/disclosure/scroll outcomes for keyboard, mouse, wheel, and
scrollbar interaction. Callers retain hierarchy, lazy-loading, filtering, and
domain state. No TableRock database concept entered TermRock.

## Bounds and failure behavior

- Rendering is limited to the visible viewport over borrowed nodes.
- Empty and tiny areas paint safely; deep indentation and wide graphemes clamp.
- Disabled state remains visible without relying on color.
- Loading and error states are semantic and reserve their rendered suffix.
- Missing selection, disabled rows, and out-of-range scroll positions clamp
  without panic or hidden I/O.

## Evidence

- Direct public-API buffer and interaction tests: 10 passing cases.
- Warmed hot-path test: 10,000 projected nodes, 40 visible rows, 100 renders,
  zero allocator/reallocator calls, 40.40 ms observed against a 250 ms batch
  gate on Apple M1 Max, macOS 26.5.2, Rust 1.97 debug profile.
- Full TermRock workspace: 200 tests pass; formatting, Clippy, rustdoc,
  packaging, semver, license/advisory/source, dependency hygiene, lookbook,
  generated preview, and docs build gates pass.
- Independent standards and specification reviews returned no findings.
- Both TermRock commits are DCO-signed, co-authored, published on `main`, and
  synchronized with `origin/main`.

This checkpoint does not claim T1 `Form`, T1 `SplitPane`, TableRock catalog
composition, or the executable shell.

External concept: generic hierarchical navigation only  
Public source: <https://github.com/tailrocks/termrock/tree/cbed106c539efbd30dbb9935863e59a7af95bef4>  
TableRock requirement: Roadmap Phase 1 / delivery-plan TermRock T1  
Implementation source: TableRock requirements, TermRock public conventions, and independent tests  
Copied code/assets/text: none
