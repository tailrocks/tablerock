# Plan 013 residual — inspector shows staged + original

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Inspect appends `staged:` / `original:` when draft present | done |
| Deleted-row draft fact on inspector | done |
| Live edit buffer line while editing | done |
| Toggle-close behavior unchanged | done |
| Unit test | done |

## Decision

Product: "the original value stays reachable." Grid paint shows staged
overlay (373); Inspect surfaces both staged and original as text so
operators recover prior values without color or hidden state.

## Evidence

```text
cargo test -p tablerock-tui --lib inspect_cursor
cargo test -p tablerock-tui --lib
```

## Remaining work

- Side-by-side original/staged compare view (optional)
