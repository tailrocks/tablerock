# Plan 013 — Cell edit session + apply effect path

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `CellEditSession` on grid (paste → buffer, Activate stages) | done |
| EditCell / DeleteRow / ApplyMutations actions | done |
| Dirty tab when drafts staged; clear on discard/success | done |
| `Effect::ApplyMutations` + CLI rebuild typed plan + authorize + apply | done |
| `DriverSession::apply_authorized_mutation` (PG only; others fail closed) | done |
| Conflict/rollback keeps staged drafts; success discards + rebrowse | done |
| Typed value editors per ValueKind | partial (text/paste buffer; bool/number via parse) |
| Registry consume-once across UI round-trip | deferred (same-effect review+authorize) |
| Admin surfaces | later |

## Verification

```text
cargo test -p tablerock-engine --lib
cargo test -p tablerock-tui --lib
cargo test -p tablerock-cli --lib
cargo test -p tablerock-engine --test postgres_real applies_authorized_update
```

## Provenance

`docs/product/editing.md` staging/apply; core mutation typestate; plan 013
apply seam (evidence 227). Clean-room.
