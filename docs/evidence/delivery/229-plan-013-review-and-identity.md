# Plan 013 — Draft→plan review + PK identity on browse

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `mutation_plan_build`: drafts → typed `MutationPlan` | done |
| Review lines: parameterized SQL + display params; never embed values in SQL | done |
| Preview never re-parsed for apply (hard product rule) | done |
| `PostgresSession::relation_primary_key_columns` | done |
| Browse fetches PK via bound statement; `GridPage.identity_columns` | done |
| Editability recompute when base table + PK present | done |
| Actions: UndoStaged, DiscardStaged, ReviewMutations | done |
| Review status line in workbench content header | done |
| Apply UI / registry authorize / cell editors | next |
| Admin (FK, table ops, activity) | later |

## Verification

```text
cargo test -p tablerock-tui --lib
cargo test -p tablerock-cli --lib
cargo test -p tablerock-engine --test postgres_real applies_authorized_update
```

## Provenance

`docs/product/editing.md` review rules; core mutation typestate; plan 012
parameter discipline. Clean-room.
