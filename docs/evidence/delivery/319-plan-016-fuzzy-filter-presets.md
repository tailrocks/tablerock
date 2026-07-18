# Plan 016 residual — fuzzy multi-preset filter picker

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `fuzzy_score` / `rank_preset_names` | done |
| `resolve_preset_name` exact + unique fuzzy | done |
| ApplyFilter confirm shows ranked matches as buffer types | done |
| Ambiguous fuzzy keeps dialog open (no silent apply) | done |
| Unit: rank, unique resolve, ambiguous keep, named round-trip | done |

## Decision

No new UI framework for a picker list: reuse confirm buffer typing.
Matches update live in the dialog body (top 8 by fuzzy rank). Submit
applies only on exact known name or a unique fuzzy hit. Ambiguous queries
stay open for refine — fail closed, no surprise load.

## Evidence

```text
cargo test -p tablerock-tui --lib filter
```

## Remaining work

- Phase 9 exit polish only if product asks for full list-navigation UI
