# Plan 013 residual — structure panel raw DDL section

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `RelationStructure` appends `-- ddl --` + CREATE TABLE text | done |
| Uses existing `compose_create_table_ddl` (presentation-side) | done |
| Unavailable → `(unavailable)` marker | done |
| Structure inspector lines use `structure:` multi-line (cap 120) | done |
| Skip empty hex block on structure panel | done |
| Unit test on RelationStructure message | done |

## Decision

Raw DDL is reconstructed from structure facts already loaded (columns /
indexes / constraints), not a second catalog query. CopyDdl remains the
clipboard path; the panel now shows the same dump for review.

## Evidence

```text
cargo test -p tablerock-tui --lib relation_structure_appends
cargo test -p tablerock-tui --lib copy_structure_ddl
cargo test -p tablerock-tui --lib inspector
```

## Remaining work

- Server-side `pg_dump --schema-only` fidelity (optional)
