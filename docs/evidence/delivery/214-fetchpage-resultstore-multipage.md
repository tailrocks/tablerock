# FetchPage + ResultStore multi-page pump

Date: 2026-07-18

## Checkpoint

Plan 009 step 2–5 (multi-page path). Browse/SQL effects pump the
PostgreSQL statement stream into a process-local `ResultStore` (up to the
10,000-row query cap, 500-row pages). The first `GridPage` is sent on the
ingress channel before the stream ends; remaining pages stay admitted for
scroll `FetchPage`. Resident scroll does not emit `FetchPage`.

## Decision

**Pump-and-store** (plan 009 STOP): the CLI effect layer cannot suspend a
held `DriverPageStream` across TEA turns without a session-scoped stream
registry. Continuous `next_page` into `ResultStore` fits the fixed budgets
(≤10k rows, 500-row pages, pin + LRU). No OFFSET re-query.

- `result_token` on `DataGridModel` is the Execute/Browse request token used
  as the `ResultId` seed.
- `FetchPage` projects an admitted page by `PageKey` and pins the viewport
  page (`set_pinned`).
- `GridStreamComplete` marks the grid completed after the background pump
  finishes (first page already painted).

## Evidence

- TUI: `resident_scroll_does_not_request_fetch`,
  `scroll_past_resident_emits_fetch_page`,
  `grid_stream_complete_marks_completed`,
  `grid_page_fills_active_tab_and_rejects_stale_context`
- Engine Docker: `browses_2500_row_table_in_500_row_pages_with_result_store_pin`
  (2,500 rows → 5×500 pages; Started+Page before Terminal; pin+get page 500)
- `cargo test -p tablerock-tui -p tablerock-cli`
- `cargo test -p tablerock-engine --test postgres_real browses_2500_row_table`

## Remaining toward Phase 4 exit

- Honest cancel terminal race labels on the TUI path (engine already proves
  `ServerConfirmedCancelled` vs completed-before-cancel).
- Ledger + ROADMAP Phase 4 exit text once cancel UI labels land.
- Structure/raw DDL tab deferred (not trivial; plan 013).
