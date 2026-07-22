# Workbench

Connecting opens the workbench: one window per session, shared by all tabs of
that connection.

## Layout

```text
+------------------------------------------------------------------+
| connection · database ▾ · schema ▾ · ENVIRONMENT · safety · health |
+--------------+---------------------------------------------------+
|              | tab · tab · tab · +                               |
|  catalog     +---------------------------------------------------+
|  filter      |                                                   |
|  ▸ schemas   |              active tab content                   |
|  ▸ tables    |        (data grid / structure / SQL editor)       |
|  ▸ views     |                                                   |
|  ▸ functions |                                                   |
+--------------+---------------------------------------------------+
| rows · timing · truncation · pending changes · focus hints       |
+------------------------------------------------------------------+
```

Hierarchy only, not geometry. Narrow terminals/windows collapse to one
region at a time: catalog becomes a drawer, the tab strip scrolls, cells
scroll instead of wrapping, and a minimum-size state replaces overlap.

## Context bar (top)

Always visible, always current:

- **Connection** — profile name.
- **Database selector** — lists databases of the session; switching changes
  the underlying connection context (PostgreSQL) or request context
  (ClickHouse), or the logical database (Redis).
- **Schema selector** — PostgreSQL only. Lists schemas of the current
  database; switching refreshes the sidebar to that schema's objects and
  retargets new tabs. Hidden where the engine has no schemas.
- **Environment tag** — the profile's environment, with `production` as a
  persistent warning treatment.
- **Safety mode** — Read only / Confirm writes.
- **Health** — connected, reconnecting, disconnected; text plus glyph.

## Sidebar catalog

The left sidebar is the object browser:

- **Filter field** on top; filtering preserves ancestor paths.
- **Tables**, **Views**, **Functions** as grouped sections per schema
  (PostgreSQL). Functions list name and argument signature. ClickHouse lists
  tables, views, dictionaries per database. Redis replaces the object tree
  with logical databases and projected key namespaces (see
  [Redis screens](redis.md)).
- Lazy expansion with explicit loading, stale, and error states per node.
- Refresh targets a subtree or the whole catalog.
- Clicking a table, view, or function opens it: tables/views open a data
  tab, functions open a definition/inspector tab.

Unsupported object kinds never render as empty sections; they show an
explicit unsupported state or are hidden by capability.

## Tabs

- **Object tabs**: opening a table creates a preview tab; editing, pinning,
  filtering, or sorting makes it durable. The same table can be open in
  several tabs at once — each tab owns its own sort, filters, columns, page
  position, and staged changes. This is the supported way to compare the
  same data under different filters side by side.
- **SQL tabs**: an editor plus results (see [SQL editor](sql-editor.md)).
  Each owns independent text, cursor, context, history, results, and errors.
- Tabs show dirty state (unsaved editor text, staged changes) and running
  state. Closing a tab with staged changes or unsaved text asks once through
  the single unsaved-change policy.
- The tab strip scrolls horizontally; every tab is reachable by keyboard.

## Status bar

Loaded rows/bytes, elapsed time, truncation, operation state
(queued/running/streaming/done/cancelled/failed), pending-change count, and
focus-aware action hints. Operation states are text, never color alone.

## Quick switcher

The workbench command opens one searchable native surface over current saved
connections, open query and object tabs, loaded catalog objects, and saved
queries. Exact and prefix title matches rank before contains matches; favorites
and pinned objects remain visibly distinguished. Activating a result rechecks
its stable identity against current model state before navigation, so stale
results fail closed. Search, list navigation, Return activation, and Escape
dismissal remain keyboard reachable.

## Both clients

| | TUI | Native macOS |
|---|---|---|
| Shell | TermRock `SplitPane`, `Tabs`, `StatusBar` | `NavigationSplitView`, toolbar, tab bar |
| Context bar | one-line context region | toolbar items with Liquid Glass treatment |
| Catalog | TermRock `Tree` | `NSOutlineView` via representable |
| Tab strip | TermRock `Tabs` | window tab bar, native gestures |

## Failure truth

- Disconnect keeps stale results inspectable and marks every live operation
  disconnected; reconnect is explicit for user queries.
- Context switches (database/schema/logical DB) invalidate dependent pages
  and completions by revision; late events for the old context are discarded.
- One failed tab never blocks other tabs.
