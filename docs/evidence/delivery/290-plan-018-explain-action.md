# Plan 018 residual — EXPLAIN action

Date: 2026-07-18

## What landed

- `ActionId::Explain` on workbench action bar
- Prefixes active editor SQL:
  - PostgreSQL: `EXPLAIN (FORMAT TEXT) …` (no ANALYZE — never auto-executes)
  - ClickHouse: `EXPLAIN …`
  - Redis: explicit unsupported error (no effect)
- Already-`EXPLAIN` text is not double-wrapped
- Empty editor fail-closed

## Commands

```bash
cargo test -p tablerock-tui explain
```

## Residual

- Structured plan parsers / tree view (still open for full plan UX)
