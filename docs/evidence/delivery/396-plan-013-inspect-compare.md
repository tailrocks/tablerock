# Plan 013 residual — inspector original|staged compare block

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `compare_original_staged` side-by-side layout | done |
| Appended on Inspect when cell draft present | done |
| Multi-line values aligned by row | done |
| Empty shown as `∅` | done |
| Unit tests | done |

## Decision

Product: original stays reachable. Lines `staged:` / `original:` remain;
`compare:` adds a fixed-width two-column table so operators scan diffs
without color alone.

## Evidence

```text
cargo test -p tablerock-tui --lib inspect_cursor
cargo test -p tablerock-tui --lib compare_original
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for compare layout
