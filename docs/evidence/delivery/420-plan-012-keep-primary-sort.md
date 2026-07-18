# Plan 012 residual — KeepPrimarySort

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `keep_primary_sort` truncate to first key | done |
| Needs ≥2 keys | done |
| Action Sort1 + rebrowse | done |
| Unit test | done |

## Decision

Multi-key ORDER BY often wants "primary only" without clearing entirely
(ClearSort) or popping one secondary at a time (PopSort). Sort1 truncates
to the primary key.

## Evidence

```text
cargo test -p tablerock-tui --lib invert_primary
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for keep-primary sort
