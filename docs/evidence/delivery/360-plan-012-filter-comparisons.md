# Plan 012 residual — Lt / Le / Gt / Ge filters

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `FilterLt` / `FilterLe` / `FilterGt` / `FilterGe` | done |
| Shared `add_value_filter` path with NE | done |
| NULL fail closed | done |
| Toolbar Lt / Le / Gt / Ge | done |
| Unit test Gt on number cell | done |

## Decision

Comparison ops reuse the cursor cell text as the bound value; engine parses
int/float/bool/text (prior browse plan builder). No expression language.

## Evidence

```text
cargo test -p tablerock-tui --lib filter_like
cargo check -p tablerock-tui
```

## Remaining work

- Raw WHERE paste dialog (optional)
