# Browse table and grid page projection bridge

Date: 2026-07-18

## Checkpoint

Plan 009 step 2 (partial). Catalog table/view leaves open a preview tab and
emit `BrowseTable` → `SELECT * FROM "schema"."table"` via `qualify_table`.
First page streams through `DriverPageRequest::PostgreSqlStatement`, projects
into `ProjectedCell` rows, and admits into the active `DataGridModel`. Stale
`context_revision` pages are rejected.

## Decision

- Identifier path uses engine `quote_ident`/`qualify_table` only.
- Page projection is row-major for VirtualGrid; kinds map to distinction
  classes without core types in TUI.
- First page only this checkpoint; scroll FetchPage deferred next.

## Evidence

- `update::tests::grid_page_fills_active_tab_and_rejects_stale_context`
- `cargo test -p tablerock-tui -p tablerock-cli`
- Log: implementer `browse-tests.log`

## Remaining work

- Scroll-driven FetchPage + ResultStore pin/evict.
- SQL tab input + Cancel full outcome labels.
- Inspector panel.
- Docker multi-page browse fixture.
