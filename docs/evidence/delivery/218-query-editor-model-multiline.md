# QueryEditorModel multiline SQL tab

Date: 2026-07-18

## Checkpoint

Plan 011 step 2. Replace single-line `tab.sql: Option<String>` with
`QueryEditorModel` (multiline text, cursor, selection, dialect-aware
statement spans, remembered editor/results split percent). View paints
TermRock `TextArea` above the result grid. Run uses selection when set,
else the current statement under the cursor.

## Decision

- Statement analysis lives in `tablerock-core` (`sql_analysis`) so pure
  presentation can recompute spans without an engine edge. Engine re-exports
  the same API. TUI may depend on `tablerock-core` (architecture diagram
  already allowed this); architecture test updated accordingly.
- Spans are recomputed outside render on every text mutation.
- `run_text()` never uses naive `split(';')`.

## Evidence

- `model::query_editor::tests::*` — selection-vs-current, paste spans,
  incomplete input, dollar-quote, split clamp
- `update::tests::run_sql_uses_selection_else_current_statement`
- `cargo test -p tablerock-tui -p tablerock-cli -p tablerock-core --lib`
- `cargo test -p tablerock-tui --test architecture`

## Remaining (plan 011)

- Keyboard routing into TextArea (cursor motion/edit keys while Content focused)
- Completion service + CompletionMenu
- History / saved queries / files / session restore
