# Query history persistence

Date: 2026-07-18

## Checkpoint

Plan 011 step 4 (persistence). Migration `0008-query-history.sql` and
`PersistenceActor` APIs: `append_history`, `list_history`, `history_count`.
Retention modes: Full (store text), MetadataOnly (NULL text), Private
(skip append). Bound default 500 rows with oldest-eviction. Never stores
result payloads.

## Decision

- Outcome class enum only (`completed` / `cancelled` / `failed` /
  `disconnected` / `unknown`) — no cell values, SQL parameters as data, or
  result pages.
- Search is substring `LIKE` on stored statement_text only (metadata-only
  rows are not text-searchable).

## Evidence

- `cargo test -p tablerock-persistence --test query_history`
  - `append_list_search_and_private_modes`
  - `enforces_bounded_row_cap` (newest-first list)

## Remaining

- TUI history panel + restore-into-editor
- Saved queries + file open/save
- Intent-only session restoration
