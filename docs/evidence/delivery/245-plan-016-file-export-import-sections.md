# Plan 016 — File effects, export, import CSV, multi-statement sections

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `AtomicFileWriter` temp+rename, abort/drop cleanup | done |
| Export CSV/JSON/TSV from loaded grid (ExportResult effect) | done |
| CSV import parse + formula-neutral cells | done |
| Multi-statement result sections model (middle failure isolation) | done |
| Streaming re-query export + full import apply | residual |
| Saved filters persistence | residual |

## Verification

```text
cargo test -p tablerock-cli --lib
cargo test -p tablerock-tui --lib result_sections
cargo test -p tablerock-tui --lib
```
