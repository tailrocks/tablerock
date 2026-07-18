# Plan 013 residual — multi-column FK follow

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `LoadForeignKeys` carries full row cells | done |
| SQL expands constraint key parts ordered by `ord` | done |
| `ForeignKeyEdge.filters` multi equality chips | done |
| Unit: row snapshot + multi-filter browse | done |

## Decision

FollowFK on any local column of a composite FK loads the whole constraint
(key parts ordered), maps each local column to the current row value, and
opens a filtered browse of the foreign table with **all** equality chips.
Single-column FKs remain a one-element filter list. First matching
constraint name wins when multiple FKs share a column.

## Evidence

```text
cargo test -p tablerock-tui --lib follow_fk_sends_full_row
cargo check -p tablerock-cli
```

## Remaining work

- Optional calendar/JSON tree widgets
- Permission-denied activity signal fixtures
