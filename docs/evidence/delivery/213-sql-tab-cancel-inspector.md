# SQL tab, cancel dispatch, and cell inspector

Date: 2026-07-18

## Checkpoint

Plan 009 steps 3–4 (partial). Workbench actions: New SQL tab, paste into
statement, Run (`ExecuteSql`), Cancel (`CancelQuery` →
`GridCancelDispatched` / cancelled state). Inspect opens
`InspectorModel` for the cursor cell with kind, bytes, truncation, text,
and hex projections.

## Decision

- Single-line SQL via tab `sql: Option<String>` until TextArea (plan 011).
- Cancel is best-effort session cancel; full race outcome labels land with
  EngineService event pump.
- Inspector does not re-fetch truncated values (honest truncation).

## Evidence

- `model::inspector::tests::inspector_marks_truncation_and_stale`
- `cargo test -p tablerock-tui -p tablerock-cli` (21 unit tests)
- Log: implementer `sql-inspector-tests.log`

## Remaining work

- ~~FetchPage on scroll + ResultStore pin.~~ → evidence 214
- Honest cancel terminal outcomes from engine pump (TUI labels still
  dispatch/cancel-requested; engine race outcomes proven in postgres_real).
- ~~Docker multi-page browse fixture.~~ → evidence 214
- Phase 4 ROADMAP exit when cancel UI race labels land.
