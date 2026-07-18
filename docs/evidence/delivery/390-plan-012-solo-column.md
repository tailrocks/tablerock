# Plan 012 residual — SoloColumn hide others

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `solo_cursor_column` | done |
| Action ColSolo | done |
| ColRst restores full layout | done |
| Unit test | done |

## Decision

Focus a wide table on the cursor column without reordering. ColRst is the
explicit restore; SaveColumns can persist a solo layout if wanted.

## Evidence

```text
cargo test -p tablerock-tui --lib solo_cursor
cargo test -p tablerock-tui --lib
```

## Remaining work

- Show all without width reset — shipped as evidence 392
