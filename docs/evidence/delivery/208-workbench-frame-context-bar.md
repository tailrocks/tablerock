# Workbench frame and context bar (plan 007 step 1)

Date: 2026-07-18

## Checkpoint

Connect opens a `WorkbenchModel` with context bar projection, welcome tab,
status summary, and catalog status placeholder. View renders these facts in
the workbench workspace (full catalog tree + tabs land next).

## Decision

- `model/workbench.rs` is TableRock-local shell state.
- `ConnectOk` builds workbench from session engine/identity/temporary flag.
- Context bar line: connection · engine · db · schema · env · safety · health.

## Evidence

- `update::tests::connect_opens_workbench_and_disconnect_returns`
- `cargo test -p tablerock-tui`

## Remaining work

- Catalog Tree + RefreshCatalog effects.
- Context switcher + revision staleness.
- Tab lifecycle + engine event pump.
