# Terminal Experience

TableRock is a terminal product, not a desktop layout compressed into cells. It
uses shared Tailrocks panels, tabs, dialogs, focus, status, hints, terminal
lifecycle, and default phosphor theme while owning database-specific layouts and
state.

## Application entry

Running `tablerock` opens the connection list. Explicit subcommands can later
list/test profiles or run bounded queries with text/JSON output without entering
the alternate-screen TUI.

Opening a connection enters its workbench. Returning to the list retains live
sessions and tab text. Quit evaluates pending edits through one modal authority.

## Focus and actions

Explicit regions:

```text
context -> catalog -> tabs -> active content -> action strip -> footer actions
```

Tab/BackTab move between regions, arrows navigate within, Enter activates,
Escape dismisses/cancels/returns predictably. The visible hint bar follows focus
and lists active actions.

Global letter shortcuts do not steal input from editors, filters, forms, or cell
editing. Every action is reachable through a visible focusable control; a
shortcut is an optimization, not the sole path.

## Responsive behavior

### Wide

- catalog beside active tab;
- editor over results with a stable focusable divider;
- complete action labels;
- grid shows complete columns and scrolls horizontally.

### Medium

- clamped catalog width;
- secondary context in status;
- overflow action list when needed;
- remembered editor/result split.

### Narrow

- one primary region at a time;
- full-height catalog drawer/screen;
- horizontally scrolling tab strip;
- separate Data/Structure/Raw screens;
- grid row gutter/active column and horizontal scrolling;
- minimum-size state instead of overlap.

## Shared versus local components

Use `tailrocks-tui` for theme, product header base, Panel, TabStrip, lists,
filters, text input, actions, status/hints, dialogs, toasts, focus, scroll, and
terminal ownership.

Keep CatalogTree, DataGrid, query editor wrapper, value inspector, change review,
and product compositions local initially. Promote a presentation-only primitive
only when a second consumer needs the same neutral state/interaction contract.

## Data grid contract

Presentation state owns selected row/column, selection mode, first visible
row/column, widths, scrollbars, and edit-marker projection. Input is one immutable
viewport with revision, global range, column metadata, resident page,
editability, and loading/truncation state.

The widget does not fetch. Near an unloaded range, update emits FetchPage.
Arrow/Page/Home/End navigation follows the active cell without resizing controls.
Enter opens inspector/editor when allowed. NULL, empty, binary, truncated, and
unknown values remain distinct. Pending changes use text/gutter plus color.

## Editor contract

Wrap the selected textarea behind TableRock state. The wrapper owns buffer,
cursor/selection, line numbers, search, scrolling, externally supplied syntax
and diagnostics, revision, and completion popup. Domain services own SQL/Redis
semantics, schema candidates, statements, parameters, and execution.

Completion is revisioned and bounded, never covers the cursor, and flips/clamps
within the editor. Old candidates cannot apply after edit/context changes.

## Operation states

Render Idle, Queued, Running before rows, Streaming, Completed rows, Completed
command, Cancel requested, Cancelled, Failed, and Disconnected deliberately.
Show elapsed time, loaded rows/bytes, truncation, cancel action/outcome, and safe
engine error context. Stale results remain inspectable when disconnected.

## Conformance

TableRock owns composition fixtures for connections, catalogs, grids, editors,
Redis values/overview, progress, failures, and write review. Test long names,
wide Unicode, minimum/normal/wide terminals, keyboard/mouse parity, non-color
cues, and loading/empty/stale/truncated/pending-change states. Never import
screenshots or assets from reference products.
