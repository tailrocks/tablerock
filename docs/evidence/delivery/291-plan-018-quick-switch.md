# Plan 018 residual — quick switcher (tabs)

Date: 2026-07-18

## What landed

- `ActionId::QuickSwitch` on workbench
- `ConfirmDialog::QuickSwitch` lists open tabs (1-based index:title)
- Paste filter: empty keeps selection; digits = 1-based index; else
  case-insensitive title substring / tab id
- No match → fail-closed label on grid

## Commands

```bash
cargo test -p tablerock-tui quick_switch
```

## Residual

- Profile/saved-query ranking: closed in evidence 299
