# Plan 012 residual — CopyCountWhereSql / CopyDistinctSql

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CopyCountWhereSql count + locator WHERE | done |
| CopyDistinctSql DISTINCT cursor column | done |
| Base-table identity required | done |
| Actions CopyCntW / CopyDist | done |
| Unit test | done |

## Decision

CopyCnt is whole-table. Point-count and distinct-value discovery are
common residual scaffolds: CopyCntW uses cursor locator WHERE; CopyDist
uses the cursor column as SELECT DISTINCT target.

## Evidence

```text
cargo test -p tablerock-tui --lib copy_count_where_and_distinct
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for count-where / distinct scaffolds
