# Filter re-browse, column layout persistence

Date: 2026-07-18

## Checkpoint

Plan 012 steps 3–4. BrowseTable carries sort/filters/raw_where; CLI builds
`BrowsePlan` and binds parameters through `PostgreSqlStatement.parameters`.
Filter actions: AddFilter (eq on cursor cell or isnull), ClearFilters.
Column actions: ToggleColumn, ResetColumns, SaveColumns; LoadColumnLayout
on browse when profile is known. Migration `0010-column-layout.sql`.

## Decision

- Parameter binding: `FilterValue` → tokio-postgres `ToSql` in
  `stream_statement`; values never concatenated into SQL.
- Column layout JSON is names/visible/width only; rejects `"cells"`.
- After LoadColumnLayout (hit or miss), rebrowse starts the data stream.

## Evidence

- `cargo test -p tablerock-persistence --test column_layout`
- `model::grid::tests::layout_json_round_trip_and_toggle`
- `update::tests::add_filter_and_cycle_sort_rebrowse_with_plan`
- `cargo test -p tablerock-tui -p tablerock-cli --lib`
- `cargo test -p tablerock-engine --lib browse_plan`

## Remaining toward plan 012 close

- Filter bar visual chips/raw-WHERE mode UI
- Column reorder/resize interaction on VirtualGrid
- Full six-format copy picker actions (CSV/TSV done; rest via format_copy)
- Ledger row updates + plan 012 DONE when residual UI is acceptable
