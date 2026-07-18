# Plan 017 residual — startup actions contract

Date: 2026-07-18

## What landed

Core types in `tablerock-core::startup_action`:

| Type | Role |
|------|------|
| `StartupSafetyClass` | ReadOnly / Write / Dangerous (`may_auto_run` only for ReadOnly) |
| `StartupAction` | bounded statement, timeout 100ms..=120s, reconnect flag |
| `StartupActionSet` | max 16 actions; filters for connect vs reconnect, auto vs review |
| `StartupActionOutcome` / `StartupRunReport` | partial-failure reporting |
| Debug | statement text redacted (byte length only) |

## Commands

```bash
cargo test -p tablerock-core startup_action
```

## Residual

- Persist set on profile aggregate
- Engine executor after connect (timeout/cancel/partial)
- TUI review for Write/Dangerous classes
