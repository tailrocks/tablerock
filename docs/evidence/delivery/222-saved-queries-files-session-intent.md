# Saved queries, atomic `.sql` files, intent-only session restore

Date: 2026-07-18

## Checkpoint

Plan 011 steps 5–6. Migration `0009-saved-queries-and-session-intent.sql`:

- `saved_queries` — named statement text per engine (upsert by name+engine)
- `session_intent` — one JSON blob per profile (tabs/context text only)

Filesystem helpers `write_sql_file_atomic` / `read_sql_file` /
`external_change_detected` live in `tablerock-persistence` (temp+rename).

TUI/CLI: SaveQuery / LoadQuery / SavedQueries panel, SaveFile, SaveIntent;
ConnectOk carries `profile_id_hex` and loads intent then catalog for
non-temporary profiles.

## Decision

- Intent JSON is hand-shaped `{database, schema, selected_tab, tabs:[{title,sql}]}`.
  Store rejects payloads containing `"cells"`, `"result_pages"`, or
  `"pending_writes"`.
- Atomic file write: same-directory temp + rename; orphan temp leaves original.
- Never store result pages or pending writes in persistence.

## Evidence

- `cargo test -p tablerock-persistence --test saved_queries_and_session_intent`
  - CRUD saved queries
  - session intent round-trip + reject result-shaped JSON
  - schema version ≥ 9
  - atomic file write + external change
- `cargo test -p tablerock-persistence --lib sql_file`
- `model::workbench::tests::intent_json_round_trip_restores_sql_tabs_not_results`
- `update::tests::save_query_and_file_emit_effects`
- `cargo test -p tablerock-tui -p tablerock-cli -p tablerock-persistence --lib`

## Remaining plan 011 polish

- File path picker / OpenSqlFile action from UI
- External-change confirm dialog on focus
- Phase 5 ROADMAP partial close after done criteria greps
