# ClickHouse Screens

ClickHouse uses the same workbench frame as PostgreSQL with analytics-shaped
behavior and honest mutation semantics.

## Sidebar and context

- The database selector lists ClickHouse databases; there is no schema
  selector.
- The catalog lists tables, views, and dictionaries per database with lazy
  loading and explicit error states.
- Table tabs show engine facts (MergeTree family, partitioning, ordering) in
  the structure view alongside columns and DDL.

## Data and queries

- The data grid, sorting, filtering, columns, copy formats, and SQL editor
  behave as specified in [Data grid](data-grid.md) and
  [SQL editor](sql-editor.md), over the official client's self-describing
  result path.
- Large results stream bounded pages with progress facts and query IDs.
  Cancellation distinguishes requested, client-stopped, server-confirmed,
  and unknown — separately visible.
- Explain variants render raw and structured plans with versioned parsing
  and unknown-node fallback.

## Writes

- **Inserts** stage and apply as progressive batches with row-confirmed
  outcomes.
- **Updates and deletes** are asynchronous mutations: the UI shows mutation
  identity and `system.mutations` status until done, failed, or unknown.
  They are never presented as transactions, instant, or rollback-capable.
- **Kill mutation** (`KillMut`) cancels one unfinished server mutation after
  re-typing the exact `mutation_id` (bound parameters only; no free SQL).
- Parts and engine operations (optimize where permitted) live behind typed
  safety gates in the administration phase.
