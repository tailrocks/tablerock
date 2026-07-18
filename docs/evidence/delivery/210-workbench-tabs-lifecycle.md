# Workbench tab lifecycle

Date: 2026-07-18

## Checkpoint

Plan 007 step 4 (tabs). Object open creates/focuses a preview tab; Pin
promotes to durable; dirty mark promotes and tracks pending count; Close
asks once when dirty; Disconnect marks health disconnected and clears
running without dropping tab content.

## Decision

- `WorkbenchModel::{open_preview_tab, promote_active_tab, mark_active_dirty,
  close_active_tab, force_close_tab, mark_disconnected}`.
- Single confirm path: `ConfirmDialog::CloseDirtyTab` reuses Submit/Cancel.
- Catalog leaf activation opens a preview tab by object name.
- Full EngineService event pump (page/stream terminals) remains for plan
  009+; status running/idle is local until then.

## Evidence

- `model::workbench::tests::preview_promotes_and_close_dirty_asks`
- `model::workbench::tests::disconnect_keeps_tabs_inspectable`
- `cargo test -p tablerock-tui` (16 unit + architecture + shell)
- Log: implementer `tabs-tests.log`

## Remaining work

- EngineService next_update pump into per-tab running/terminal messages.
- Schema selector picker UI.
- Real-server catalog sidebar fixture.
