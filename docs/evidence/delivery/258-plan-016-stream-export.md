# Plan 016 residual — streaming re-query export + cancel cleanup

Date: 2026-07-18

## What landed

- `crates/tablerock-cli/src/stream_export.rs`
  - `StreamExporter` / `run_stream_export` for CSV/TSV/JSON
  - Atomic temp+rename; cancel aborts and removes temp (no dest)
- Unit tests: multi-page CSV finish, mid-stream cancel cleanup, JSON array shape
- `Effect::ExportStreamQuery` + `export_stream_query` executor (PG/CH page stream → file)
- Real Docker test: `stream_export_real` re-queries `export_probe` with page size 2

```bash
cargo test -p tablerock-cli stream_export
cargo test -p tablerock-cli --test stream_export_real
```

## Residual

- TUI action wiring for stream-export path (effect is ready)
- Redis export remains unsupported (explicit reject)
