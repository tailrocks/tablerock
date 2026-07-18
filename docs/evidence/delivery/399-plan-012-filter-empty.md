# Plan 012 residual — FilterEmpty / FilterNotEmpty

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| FilterEmpty → `eq` with empty string | done |
| FilterNotEmpty → `ne` with empty string | done |
| Distinct from IS NULL / IS NOT NULL | done |
| Actions Empty / NotEmpty | done |
| Unit test | done |

## Decision

Empty string and SQL NULL are different truths. Operators need both filters
without raw WHERE. Empty binds as text `''` through existing eq/ne chips.

## Evidence

```text
cargo test -p tablerock-tui --lib filter_null
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for empty-string filters
