# Plan 018 residual — type-specific cell edit affordances

Date: 2026-07-18

## What landed

- `CellEditSession.kind` captures distinction at edit start
- `toggle_boolean` for Boolean cells (`true` ↔ `false`)
- `set_null` for explicit NULL token
- Actions: `ToggleBool`, `SetNull` on workbench bar during edit
- Existing: distinction-gated commit + `parse_staged_value` (bool/number/null/text)

## Commands

```bash
cargo test -p tablerock-tui boolean_toggle
```

## Residual

- Dedicated temporal/JSON/bytes widgets: inspector projections in evidence 300;
  full tree/calendar widgets still optional polish
