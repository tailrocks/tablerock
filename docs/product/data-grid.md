# Data Grid

Clicking a table or view in the sidebar opens the data grid. The same grid
renders arbitrary SQL results; table browsing adds editing, sorting, and
filtering.

## Browsing

- Rows arrive in bounded server pages (default 500); the grid renders only
  resident rows around the viewport. Scrolling resident data performs no I/O.
- Totals may be exact, estimated, or unknown; the status bar says which.
- Every cell renders its typed distinction: NULL, empty, whitespace, zero,
  false, binary, structured, truncated, unknown — visibly different, never
  color alone.
- A row/cell inspector shows the full typed value with text, JSON, and hex
  projections, plus metadata and stale state.

## Sorting

- Clicking a column header cycles ascending, descending, none; a second
  column adds a tie-breaker. The active sort is visible per column with its
  order index.
- Sorting runs on the server with parameterized identifiers — never string
  concatenation — and re-fetches pages. Sort provenance is shown in the
  status bar and cleared on reset.

## Filtering

A filter bar above the grid offers two modes:

1. **Column filters** — one row per condition: column picker, operator list
  typed to the column type (`=`, `≠`, `<`, `>`, `contains`, `is null`, …),
  value input. Conditions combine with AND. Adding, editing, and removing a
  condition re-runs the query.
2. **Raw WHERE** — a free SQL fragment appended to the browse query, for
  expressions the column UI cannot form. Values stay parameterized where the
  fragment references them; hostile fragments fail closed.

Both modes show as removable chips/rows with a one-action **clear all**.
A separate quick filter searches only resident rows and is visibly labeled
as page-local — never confused with server filtering.

Saved filter presets: **SaveFilt** prompts for a preset name and stores the
current server filters for the active table on the connected profile;
**LoadFilt** prompts for a name (with known-name hints) and re-browses.
Libraries load on connect (non-temporary profiles).

## Columns

- Show/hide any column, reorder by drag or keyboard, resize widths.
- **Reset** restores the default set and order.
- Column layout persists per table across sessions.
- Hiding columns never changes the underlying query identity or editability.

## Operation states

The grid deliberately renders: idle, queued, running before rows, streaming,
completed, cancel requested, cancelled, failed, disconnected — with elapsed
time, loaded rows/bytes, truncation, and a cancel action while running.
Failed loads keep stale pages visible and marked stale.

## Both clients

| | TUI | Native macOS |
|---|---|---|
| Grid | TermRock `VirtualGrid` + local `DataGridModel` | `NSTableView` via representable |
| Filter bar | focusable rows under the tab strip | toolbar-attached filter row |
| Sort headers | header cells with glyph + index | native column header sort indicators |
| Inspector | side/bottom panel | side panel or popover |

## Failure truth

- Sort/filter on hostile identifiers cannot alter query structure.
- Unknown or unsupported column types remain inspectable, never silently
  editable, and offer only valid filter operators.
- Cancel stops page fetching; already-loaded rows stay inspectable.
