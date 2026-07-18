# Plan 013 residual — copy CREATE TABLE DDL from structure

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `compose_create_table_ddl` from structure panel lines | done |
| Columns + CONSTRAINT + trailing CREATE INDEX statements | done |
| `ActionId::CopyStructureDdl` → OSC 52 `CopyToClipboard` | done |
| Fail closed without open structure target / columns | done |
| Unit tests for compose + action dispatch | done |

## Decision

Copy is presentation-side reconstruction of structure facts already loaded
into the inspector (columns / indexes / constraints). No second schema
model; no free SQL edit path. Identifier quoting is double-quote only.

## Evidence

```text
cargo test -p tablerock-tui --lib structure_ddl
cargo test -p tablerock-tui --lib copy_structure_ddl
cargo check -p tablerock-tui
```

## Remaining work

- Optional: `pg_dump --schema-only` fidelity for edge types
