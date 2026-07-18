# Plan 013 residual — InsertRow + DuplicateRow staging

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `stage_insert_blank` (all columns empty → NULL) | done |
| `stage_insert_from_cursor` (copy presentation values) | done |
| Actions InsRow / DupRow | done |
| Dirty tab on stage | done |
| Status `staged N (↑ · ↓)` includes inserts | done |
| ReadOnly fail closed | done |
| Unit test | done |

## Decision

Product "Add a row" stages an in-memory insert only (never touches the
server until Review/Apply). Blank insert leaves generated/default columns
empty so the database invents them. Duplicate copies cursor presentation
text as a template; identity conflicts stay an apply-time truth.

Insert drafts still surface in Review/Apply via existing draft plan
builders; inline grid-row editing of insert drafts remains optional.

## Evidence

```text
cargo test -p tablerock-tui --lib stage_insert
cargo test -p tablerock-tui --lib
cargo check -p tablerock-tui
```

## Remaining work

- Inline editable inserted-row viewport — paint shipped as evidence 386; cursor focus optional
- Resident-row draft paint for updates/deletes — shipped as evidence 373
- EditInsert col=value dialog — shipped as evidence 377
