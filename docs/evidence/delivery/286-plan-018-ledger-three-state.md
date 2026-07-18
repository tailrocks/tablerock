# Plan 018 — ledger three-state closure pass

Date: 2026-07-18

## Closure rule (from functional-parity-ledger.md)

Every row ends as one of:

1. **implemented** — tests + docs linked
2. **excluded** — fixed product-boundary decision
3. **gap** — visible blocker for the corresponding claim

Machine-readable export:
[`286-plan-018-ledger-three-state.csv`](286-plan-018-ledger-three-state.csv)

## Counts (this pass)

| three_state | count |
|-------------|-------|
| implemented | 57 |
| excluded | 7 |
| gap | 1 |

### Gap rows (block claims as noted)

| Capability | Sequencing | Blocks |
|------------|------------|--------|
| Multi-window | Native | Native (plan 019–021) |

## Parity claim status

**TUI Core + Parity rows are implemented or excluded** in the three-state
CSV (this pass). The remaining **gap** is Native multi-window, blocked on
plan 019 packaging (full Xcode + Developer ID + notarization).

No marketing “full product complete” claim until 019–021 exit.

## Commands

```bash
# CSV is the export artifact; counts above derived by review of ledger rows.
wc -l docs/evidence/delivery/286-plan-018-ledger-three-state.csv
```
