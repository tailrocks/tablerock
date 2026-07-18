# Plan 017 residual — structure-panel DDL quick actions

Date: 2026-07-18

## What landed

- `InspectorModel.structure_schema` / `structure_table` retain the
  relation when structure loads
- Structure body includes a quick-actions legend (AddCol / DropCol /
  AddIdx / DropIdx / AddCon / DropCon)
- `relation_ddl_target` resolves grid base first, else structure target
- All six DDL action bar entries use `open_ddl_review` via that target
- Unit: `structure_panel_target_enables_ddl_without_grid_base`

## Commands

```bash
cargo test -p tablerock-tui structure_panel
cargo test -p tablerock-tui ddl_add
```

## Residual (plan 017)

- Full pg_dump/pg_restore real-server matrix when CI has client binaries
