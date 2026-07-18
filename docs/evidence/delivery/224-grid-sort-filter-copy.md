# Grid sort state, page-local filter, copy formats

Date: 2026-07-18

## Checkpoint

Plan 012 steps 2–5 (partial). `DataGridModel` gains sort keys, typed filter
chips, raw WHERE, page-local quick filter, column layout, and base-table
identity for SQL copy. Pure `copy_format` module: CSV/TSV/JSON/Markdown/
SQL INSERT/UPDATE with identity gating. `CopyToClipboard` effect emits
OSC 52 to stdout. `CycleSort` cycles the cursor column; re-browse when base
table is known.

## Decision

- Quick filter never emits effects (page-local only; status label
  `page-local filter`).
- SQL INSERT/UPDATE refuse without `base_schema` + `base_table` (browse sets them).
- OSC 52 written directly from CLI effect (TermRock typed OSC API not
  required for this checkpoint; STOP avoided).

## Evidence

- `model::copy_format::tests::*`
- `model::grid::tests::cycle_sort_and_quick_filter_page_local`
- `cargo test -p tablerock-tui --lib`
- Browse plan builder: evidence 223

## Remaining (plan 012)

- Filter bar UI + re-run via BrowsePlan parameters
- Column show/hide/reorder/resize persistence migration
- Full six-format action picker + golden files
- Header hit-region sort clicks on VirtualGrid
