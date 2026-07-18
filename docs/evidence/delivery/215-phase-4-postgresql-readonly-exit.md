# Phase 4 PostgreSQL read-only vertical slice exit

Date: 2026-07-18

## Checkpoint

Plan 009 / ROADMAP Phase 4 exit. PostgreSQL read path on trunk: catalog,
context bar, tabs, browse + SQL streaming, typed grid, inspector, cancel
dispatch vs observed outcome, multi-page ResultStore pin, first rows before
completion, resident-scroll no-I/O.

## Decision

Phase 4 is closed for the read-only vertical slice. Deferred (explicit):

- Structure/raw DDL tab (plan 013)
- Server sort/filter/column controls (plan 012)
- EngineService event-pump cancel race matrix on the TUI path (engine
  `postgres_real` already proves ServerConfirmed vs completed-before-cancel;
  TUI shows cancel-requested vs cancelled + observed label from stream errors)
- Pre-existing clippy lints in `tablerock-tui` (not introduced by 009)

**Pump-and-store** for multi-page (evidence 214): continuous `next_page` into
`ResultStore` up to 10k rows; FetchPage projects admitted pages; no OFFSET
re-query.

## Evidence chain

| # | Topic |
|---|---|
| 208 | Workbench frame + context bar |
| 209 | Catalog tree + context revision |
| 210 | Tab lifecycle |
| 211 | DataGridModel + quote_ident + VirtualGrid |
| 212 | Browse table first page |
| 213 | SQL tab, cancel, inspector |
| 214 | FetchPage + ResultStore multi-page |

## Delivery-plan exit mapping

| Exit item | Proof |
|---|---|
| First rows before completion | First `GridPage` on ingress before pump ends; EngineService Started+Page before Terminal (Docker 2500-row) |
| Stale pages/events cannot cross context revision | `grid_page_fills…rejects_stale_context` |
| Resident scroll no I/O | `resident_scroll_does_not_request_fetch` |
| Caps exact | `MAX_QUERY_ROWS=10_000` in CLI pump; 500-row pages |
| Unknown inspectable, non-editable | Inspector + no edit affordance (editing is plan 013) |
| Cancel race truth | CancelRequested vs Cancelled+label; engine cancel races |

## Verification

- `cargo test -p tablerock-tui -p tablerock-cli`
- `cargo test -p tablerock-engine --test postgres_real browses_2500_row_table`
