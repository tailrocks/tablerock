# Plan 013 residual — structured tree expand/collapse

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `tree_source` + `tree_depth` on inspector | done |
| `pretty_structured_depth` parameterized | done |
| ExpandTree / CollapseTree actions | done |
| Depth range 1..=32 | done |
| Unit test | done |

## Decision

Default depth remains 6. Operators raise/lower nesting with Tree+/Tree-
without re-fetching the cell. Collapsed levels still show `…` so deep
documents stay bounded.

## Evidence

```text
cargo test -p tablerock-tui --lib expand_and_collapse
cargo test -p tablerock-tui --lib
```

## Remaining work

- Cursor-driven path expand of one key (optional)
