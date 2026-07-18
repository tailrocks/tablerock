# Plan 016 residual — TUI wiring for import + stream export

Date: 2026-07-18

## What landed

### Actions
- `ActionId::ExportStreamCsv|Json|Tsv` → `Effect::ExportStreamQuery` (editor SQL)
- `ActionId::ImportCsv` → `Effect::ImportCsvApply` (base table + `import.csv`)

### CLI
- `Effect::ImportCsvApply` handler reads CSV path, `parse_csv`,
  `apply_csv_inserts` through mutation review seam; reports
  `MutationApplied` / `MutationFailed`

### View
- Workbench actions: ExpStream, ImpCsv

### Tests
```bash
cargo test -p tablerock-tui --lib export_stream_emits
cargo test -p tablerock-tui --lib import_csv
# 3 passed
```
