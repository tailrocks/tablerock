# Native macOS information architecture

## Shared object vocabulary

The production app uses one stable hierarchy across engine-specific surfaces:

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

## Workbench structure

Native Workbench uses a persistent leading catalog, compact context toolbar,
document tabs, opaque work area, bottom status rail, and optional trailing
inspector. Query workspaces keep editor and results in one vertical continuum.

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
