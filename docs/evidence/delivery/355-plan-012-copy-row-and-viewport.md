# Plan 012 residual — copy row + million-row viewport model

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `CopyRow` → TSV of cursor row via `CopyScope::Row` | done |
| Toolbar CopyRow | done |
| Synthetic 1e6 total with 100-row resident page | done |
| `is_resident` / `needs_fetch` O(1) far from window | done |
| No per-total cell allocation | done |

## Decision

Copy row reuses existing formatters (TSV). Million-row proof is a pure model
unit test: totals may be Exact(1_000_000) while `cells` length stays at the
resident page size.

## Evidence

```text
cargo test -p tablerock-tui --lib format_cursor
cargo test -p tablerock-tui --lib million_row
cargo check -p tablerock-tui
```

## Remaining work

- Live VirtualGrid frame-time microbench in CI (optional Phase 11)
