# Plan 016 residual — CSV import apply through mutation seam (PostgreSQL)

Date: 2026-07-18

## What landed

- `tablerock_cli::apply_csv_inserts`: parse path → `csv_to_insert_changes` →
  `MutationPlan` → consume-once review registry →
  `DriverSession::apply_authorized_mutation`
- **No SQL string concatenation** on the import path (plan 016 STOP)
- Formula-like cells remain text field values
- Real Docker test: `crates/tablerock-cli/tests/import_apply_real.rs`
  - Creates `csv_import_probe(id text, label text)`
  - Imports two rows including `=SUM(A1)` as data
  - Asserts `Committed` and two `Applied { rows_affected: 1 }`

```bash
cargo test -p tablerock-cli import
cargo test -p tablerock-cli --test import_apply_real
```

CI: real-servers job runs `import_apply_real`.

## Residual

- ClickHouse import apply
- TUI Effect wiring for operator-facing import
