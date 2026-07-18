# Plan 012 residual — LIKE / ILIKE / NE filters

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `FilterLike` wraps cell text as `%value%` when no `%` present | done |
| `FilterILike` same with `ilike` op | done |
| `FilterNe` inequality with raw cell text | done |
| NULL / empty cell fail closed (no effect) | done |
| Toolbar Like / ILike / NotEq | done |
| Unit test | done |

## Decision

LIKE patterns auto-wrap with `%` for “contains” UX; operators who already
typed wildcards keep them. Engine maps `like`/`ilike`/`ne` (prior).

## Evidence

```text
cargo test -p tablerock-tui --lib filter_like
cargo check -p tablerock-tui
```

## Remaining work

- Numeric comparison (lt/gt) actions from toolbar (optional)
