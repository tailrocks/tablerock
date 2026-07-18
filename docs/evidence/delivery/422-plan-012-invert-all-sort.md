# Plan 012 residual — InvertAllSort

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `invert_all_sorts` Asc↔Desc every key | done |
| Empty sort no-op | done |
| Action SortInvA + rebrowse | done |
| Unit test | done |

## Decision

SortInv flips only the primary key. Multi-key reverse browsing needs every
key flipped in place (stable order, inverted directions). SortInvA does that.

## Evidence

```text
cargo test -p tablerock-tui --lib invert_all_sorts
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for invert-all sort
