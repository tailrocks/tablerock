# Native macOS information architecture

## Shared object vocabulary

The lab uses one stable hierarchy so concept comparisons measure layout rather
than changed information:

```text
Connection group
└── Connection (environment, engine, health)
    └── Database
        └── Schema
            ├── Tables
            ├── Views
            ├── Functions
            └── Types
                └── Object
                    ├── Data
                    ├── Structure
                    └── Relations
```

Queries are connection-scoped tabs. Results belong to a query execution.
Pending changes belong to one connection/database transaction context and are
reachable from every editing surface.

## Global command hierarchy

1. Window/app commands: new window, settings, help.
2. Connection context: current connection, database, environment, reconnect.
3. Workspace commands: object/query tabs, search, navigation visibility.
4. Surface commands: run, filter, sort, edit, export, inspect.
5. Safety commands: safe mode, pending-change review, discard, apply.

Destructive and transactional commands never share an unlabeled icon cluster
with ordinary navigation.

## Concept structures

| Concept | Primary structure | Best question answered |
|---|---|---|
| 1. Native Workbench | Persistent leading catalog + toolbar/context capsule + tab strip + opaque work area + status rail + optional inspector | Can TableRock adopt the operator's preferred TablePro rhythm faithfully? |
| 2. Query Studio | Narrow mode rail + editor-led vertical split + docked results/console | What if SQL composition is the spatial anchor? |
| 3. Column Observatory | Three persistent columns: sources, objects, detail | What if browse depth remains visible instead of living in one tree? |
| 4. Grid Canvas | Edge-to-edge content with detached navigator and command palettes | What if maximum data area is primary? |
| 5. Change Desk | Compact navigator + central work area + persistent change ledger | What if write safety is always spatially present? |

These are structural alternatives. Color, fixtures, typography, and shared
components remain intentionally consistent.

## Surface hierarchy

- Connections starts at the connection collection; no database catalog shell
  is implied before connection.
- Connection Setup is a focused form with a live summary and separated
  advanced transport/security disclosure.
- Data Grid makes object identity, selection, filters, row count, page, and
  pending state continuously available.
- SQL + Results preserves editor/result ownership when resizing either region.
- Change Review shows semantic changes before generated SQL and places the
  final apply action after the review sequence.
