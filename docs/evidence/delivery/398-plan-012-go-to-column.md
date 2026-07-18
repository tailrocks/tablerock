# Plan 012 residual — GoToColumn by name

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ConfirmDialog::GoToColumn` | done |
| Exact name match | done |
| Unique case-insensitive prefix | done |
| Ambiguous/missing fail closed | done |
| Unhides target if layout-hidden | done |
| Action GoCol | done |
| Unit test | done |

## Decision

Wide tables need named jump without scanning ColR. Exact wins over prefix;
ambiguous prefixes refuse to move the cursor.

## Evidence

```text
cargo test -p tablerock-tui --lib go_to_column
cargo test -p tablerock-tui --lib
```

## Remaining work

- Fuzzy unique match beyond prefix (optional)
- Viewport reveal after jump — shipped as evidence 406
