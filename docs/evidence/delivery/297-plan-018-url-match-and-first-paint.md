# Plan 018 residual — external URL profile match + first-paint budget

Date: 2026-07-18

## External URL → matching saved session

When the loaded profile list has a row with the same engine label and
`host:port/database` target summary as the parsed URL:

1. Summary shows `matched saved profile 'name'`
2. OPEN dispatches `ConnectProfile` (not temporary draft connect)
3. No match → temporary `ConnectSession` as before

Units:

- `open_external_url_requires_open_then_connects_temporary`
- `open_external_url_matches_saved_profile`
- `open_external_url_rejects_hostile_scheme`

## First-paint budget (unit)

`crates/tablerock-tui/tests/shell.rs::first_paint_budget_under_50ms`

| Metric | Budget | Path |
|--------|--------|------|
| Cold Model + Resize(100×30) + ShellView draw | &lt; 50 ms | TestBackend unit |

Local-rig only (not fixed-spec CI). Complements process-start budget in evidence 288.

## Commands

```bash
cargo test -p tablerock-tui open_external
cargo test -p tablerock-tui --test shell first_paint
```
