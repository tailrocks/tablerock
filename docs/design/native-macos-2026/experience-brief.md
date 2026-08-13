# Native macOS experience brief

Status: production design contract
Audience: operators and implementers

## Experience promise

TableRock should feel like a focused native database instrument: immediate,
dense without becoming cramped, keyboard-complete, and calm around dangerous
actions. TablePro's public macOS composition is the lead reference because the
operator explicitly prefers its design. TableRock adapts the familiar
workbench rhythm while retaining its own product identity and safety model.

The primary working loop is:

1. choose or create a connection;
2. orient within server, database, schema, and object;
3. inspect data or compose SQL without losing context;
4. stage edits visibly;
5. review exact effects and apply intentionally.

## Experience principles

- **Content earns the space.** Grids, SQL, and diffs are opaque, stable, and
  visually dominant. Navigation and controls support them.
- **Context never becomes a mystery.** Environment, connection, database,
  schema, object, query tab, and pending-change count remain discoverable.
- **Native before novel.** Use standard macOS toolbar, sidebar, search, sheet,
  menu, focus, selection, and keyboard behaviors wherever they fit.
- **Glass has a job.** Liquid Glass marks navigation and top-level controls. It
  never coats the data grid, SQL editor, inspector body, or review content.
- **Safety is semantic.** Production state and write risk use labels, icons,
  confirmation, and wording—not color alone.
- **Density is operator-controlled.** The default fits serious data work while
  preserving legible hit targets and system text scaling.
- **Every state has an answer.** Loading, empty, disconnected, stale, error,
  read-only, and pending-change states need explicit presentation contracts.

## Required production surfaces

- Connections: groups, environment labels, recency, health, add/search actions.
- Connection Setup: engine, host, port, database, identity, SSL/SSH, safe mode,
  test, save, and connect hierarchy.
- Data Grid: catalog context, object tabs, filters, sort, columns, row selection,
  paging, status, and an optional value inspector.
- SQL + Results: query tabs, editor, execute controls, duration, row count,
  result grid, and messages.
- Change Review: staged inserts/updates/deletes, before/after values, SQL
  preview, risk context, discard, and apply.

## Accessibility acceptance

- Complete keyboard traversal and visible focus.
- System semantic colors and fonts; no essential meaning from color alone.
- Labels for icon-only controls and useful grouping for dense grid regions.
- Layout remains intelligible under Increase Contrast.
- Reduce Transparency replaces glass with a strong opaque material.
- Reduce Motion removes decorative transitions and morphing.
- Light and dark appearances both retain hierarchy and readable separators.
