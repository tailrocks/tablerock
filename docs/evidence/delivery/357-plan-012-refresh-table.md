# Plan 012 residual — RefreshTable re-browse

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ActionId::RefreshTable` | done |
| Reuses `rebrowse_active_table` (keeps sort/filters) | done |
| Toolbar Refresh | done |
| Unit test preserves filter chips on BrowseTable | done |

## Decision

Refresh is explicit operator re-query of the base table identity. Distinct
from ClearFilters (which drops server controls first). No cache of prior
pages across refresh — engine fetch is authoritative.

## Evidence

```text
cargo test -p tablerock-tui --lib refresh_table
cargo check -p tablerock-tui
```

## Remaining work

- Auto-refresh interval preference (optional; HealthTick is separate)
