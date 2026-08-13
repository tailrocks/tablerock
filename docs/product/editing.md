# Editing And Review

Table data is editable in place. Nothing touches the database until the
operator previews and applies: every change is staged in memory first.

## Change Ledger (product name)

The in-memory staged set is the **Change Ledger**: a calm, reviewable list of
proposed inserts, updates, and deletes for the active tab. The status bar
counts ledger entries; the review dialog shows exact parameterized statements
before apply. Discard and undo clear ledger entries without touching the
server. This is the same staging model as below — the name makes consequence
visible in the UI.

## Editability

A result is editable only when it comes from one base table with stable row
identity (primary or unique key). Joins, aggregates, and key-less results are
read-only and say why. Unknown or truncated values are never editable.

The first native production path is deliberately narrower: a selected
PostgreSQL base-table row is editable only after Rust proves a primary-key
identity from the catalog browse result. Identity columns, null cells, binary
cells, and values that are unknown, invalid, structured, or truncated are not
offered by that sheet. Unique-key-only relations and richer native value
editors remain part of the broader parity requirement; presentation code must
not guess their semantics from engine type names.

## Staging

- **Edit a cell**: inline editor typed to the column (bool, number, temporal,
  enum, JSON, bytes, …). Accepting stages the change; it does not execute.
- **Native selected-row update**: Edit Selected opens a typed sheet for the
  writable values in one PostgreSQL row. Stage for Review freezes only changed
  fields; Back revokes the old authority before returning to the draft.
- **Add a row**: a new editable row appears in the grid; default values and
  generated columns are marked, not invented.
- **Delete rows**: selected rows stage for deletion.
- Staged changes survive scrolling, paging, sorting, and filtering inside
  their tab. They belong to the tab that created them.

### Highlighting

Pending state is visible at a glance, with text/gutter markers plus color —
never color alone:

- **Inserted** rows: added-row marker and distinct treatment.
- **Modified** rows/cells: changed-cell marker; the original value stays
  reachable.
- **Deleted** rows: struck-through treatment; they remain visible until
  apply or discard.

The status bar counts pending changes per tab; the tab strip marks tabs
carrying staged work.

## Preview before apply

**Review** opens a dialog listing the exact operations the apply will run,
grouped by table, each as a parameterized statement with its values:

```text
INSERT INTO public.orders (id, status, total) VALUES ($1, $2, $3)   -- new row
UPDATE public.orders SET status = $1 WHERE id = $2                  -- 1 row
DELETE FROM public.users WHERE id = $1                              -- 2 rows
```

The preview is descriptive. Execution always uses the typed plan behind it —
never reparsed preview text. The operator can apply everything or discard
everything; per-change discard is available from the staged view.

## Apply and outcome

- **PostgreSQL**: one tab's change set applies in a single transaction.
  Zero/multiple affected rows mean conflict: roll back, report, keep staged
  state for resolution. Generated values (serial, defaults) reconcile after
  apply. The grid refreshes from the server.
- **ClickHouse**: inserts are progressive batches; updates/deletes are
  asynchronous mutations tracked to done/failed/unknown. The UI never calls
  them transactions.
- **Redis**: changes are sequential type-specific commands with exact TTL
  effects shown in review; no rollback is claimed.
- An ambiguous outcome (dispatch without observed result) stays **unknown**,
  is never silently retried, and is recorded.

After a successful apply, staged markers clear and the grid reloads the
affected pages.

## Unsaved-change policy

Closing a tab, switching context, disconnecting, or quitting with staged
changes routes through one modal authority: apply, discard, or cancel the
action. No path silently drops staged work.

## Safety modes

- **Read only** profiles cannot stage changes; edit affordances are absent,
  not just disabled.
- **Confirm writes** profiles stage freely and require the explicit
  review-and-apply step above.
- Destructive operations (drop, truncate) are separate reviewed gates, not
  part of cell editing.

## Table operation authority

Table operations start from an opaque catalog selection, not editable SQL or
presentation text. PostgreSQL tables offer rename, truncate, drop, vacuum, and
analyze; ClickHouse tables offer optimize. Unsupported engine/object pairs do
not expose an operation.

Review freezes the target and exact quoted statement for 60 seconds. Rename
requires a non-empty new name. Every operation requires the exact target name;
truncate and drop are additionally marked destructive. Apply consumes the
review token before database I/O, so expiry, cancellation, or a second apply
cannot replay it. Wrong confirmation leaves the token available for correction.
Successful rename/drop refreshes the catalog and closes stale object state;
truncate reloads the selected table.

PostgreSQL vacuum and analyze execute as Rust-owned background operations.
Their sheet remains visible while running, reports the terminal success or
failure outcome, and cannot be dismissed mid-operation. Cancellation is shown
as unavailable because the reviewed-DDL adapter does not yet expose a truthful
cancel boundary; the UI must never present a cancel action that only abandons
observation.

## Both clients

| | TUI | Native macOS |
|---|---|---|
| Cell editor | in-grid typed editors | selected-row typed sheet for PostgreSQL updates; richer in-cell parity remains |
| Markers | gutter glyphs + row treatment | row background + badge, label on focus |
| Review dialog | modal with `DiffView`-style list | **Change Review** sheet plane: kind · monospaced preview · DESTRUCTIVE word · expiry · Apply/Discard |
| Apply | explicit action + shortcut | Apply inside review only (consume-once token); never silent write from chrome |
| Ledger chrome | status `ledger N` / **Ledger** action | status `LEDGER N` + fact chip when any review open |
