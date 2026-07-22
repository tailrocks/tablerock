# Native layout and hit-testing containment

Date: 2026-07-22

## Hosted evidence

Native checkpoint `29878692056`, attempt 2, at exact commit `21cf1cc`
completed all pre-Xcode gates. Its partial canonical Xcode run proved:

- feature, bridge, and application-model suites passed;
- CSV Stage identifier remained absent;
- grid selection still did not open the inspector;
- export click fell back to `{429, 681}`;
- quick-filter frame was `{{233, 663}, {222, 26}}` while the workbench window
  ended at y=654, so typing failed because the visible semantic node was
  outside the hit-testable window.

The run was externally cancelled after these failures; it is evidence of the
failures, not a complete checkpoint verdict.

## Structural correction

- Result grids now treat requested heights as ideals with a bounded 100-point
  minimum. Toolbars remain inside the window instead of overflowing below a
  rigid grid minimum.
- CSV sheets use ideal rather than rigid minimum height.
- Styled buttons receive stable identifiers before style wrapping; export
  buttons own their style instead of inheriting it from a container.
- AppKit result cells expose an actionable button role and native press action,
  while retaining row/column label, value, and stable identifier.

No timeout, expected result, file assertion, safety rule, or backend behavior
was weakened.

## Local verification

```text
(cd native && swift build -c release)
# production package compiled

xcrun swiftc -parse native/Tests/TableRockAppUITests/TableRockAppUITests.swift
# canonical UI suite parsed

git diff --check
# clean
```

Exact-main hosted Xcode proof remains required.

## Provenance

No external product source, test, identifier, text, asset, screenshot, layout
measurement, color, or key binding influenced this correction. It derives
solely from TableRock's hosted accessibility event frames, AppKit semantics,
product layout contract, and plan 021.
