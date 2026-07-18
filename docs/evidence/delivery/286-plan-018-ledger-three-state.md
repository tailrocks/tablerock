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
| implemented | 53 |
| excluded | 7 |
| gap | 7 |

### Gap rows (block claims as noted)

| Capability | Sequencing | Blocks |
|------------|------------|--------|
| Find/replace | Parity | Parity claim |
| Formatting | Parity | Parity claim |
| Explain | Parity | Parity claim |
| Type-specific editors | Parity | Parity claim |
| External URL open | Later | Later only |
| Multi-window | Native | Native (plan 019–021) |

## Parity claim status

**No marketing parity claim.** Core TUI workflows are largely implemented.
Remaining **Parity** gaps are explicit and block a full three-engine parity
claim until closed or reclassified.

Native claim blocked on plan 019 packaging (Xcode + Developer ID).

## Commands

```bash
# CSV is the export artifact; counts above derived by review of ledger rows.
wc -l docs/evidence/delivery/286-plan-018-ledger-three-state.csv
```
