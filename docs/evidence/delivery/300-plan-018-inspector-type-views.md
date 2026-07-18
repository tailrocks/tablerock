# Plan 018 residual — type-aware inspector projections

Date: 2026-07-18

## What landed

`InspectorModel::from_cell` kind-specific text:

| Distinction | Projection |
|-------------|------------|
| Structured | Best-effort JSON-like pretty-indent |
| Temporal | ISO component lines (date/time/fraction/tz) |
| Boolean | value + TogBool/SetNull hint |
| Binary | note that hex panel holds first bytes |

Units: `structured_pretty_print_and_temporal_annotation`, fallback for invalid JSON.

## Commands

```bash
cargo test -p tablerock-tui inspector
```
