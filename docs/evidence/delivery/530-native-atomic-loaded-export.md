# Native atomic loaded-result export

Date: 2026-07-19

## Shared ownership

The atomic file writer moved from the CLI adapter into the internal
`tablerock-files` effect crate so TUI and native paths use one same-directory
temp, flush, fsync, and rename policy without placing I/O in core contracts.
Exclusive per-writer temp names prevent concurrent writers from sharing or
deleting another writer's incomplete file. Drop, abort, path rejection, write
failure, overwrite, and concurrent-writer cleanup remain directly tested.

## Native behavior

The result toolbar exposes CSV, TSV, JSON, Markdown, and identity-gated SQL
INSERT export through `NSSavePanel`. Swift balances security-scoped access;
UniFFI accepts only an absolute destination and formats resident typed pages in
Rust before atomic replacement. Output remains bounded by the shared 10,000-row,
1,024-column, 16 MiB copy projection limits.

Re-audit found object-tab copy/export routing could read the selected query
tab's result handle. Active result ID and revision now switch with the selected
workbench kind, removing cross-tab result leakage.

## Evidence

- Full core, CLI, and FFI suites: pass.
- Core/FFI Rust 1.97 clippy with warnings denied and dependencies excluded:
  pass.
- FFI conformance: atomic bytes exactly match shared CSV formatting; relative
  native paths fail closed.
- Live PostgreSQL native fixture: JSON export contains typed numeric output,
  exact byte count agrees, destination exists only after completion, and no
  `.tablerock-tmp-*` residue remains: pass.
- Native object-tab and accessibility structural/runtime regressions: pass.

## Remaining boundary

This checkpoint exports all currently resident rows. Full pull-driven
constant-memory export beyond resident pages, progress/cancellation UI, and
import screens remain open and are not claimed here.

## Provenance

TablePro was used only to confirm the broad save-panel export workflow. No
source, tests, text, screenshots, layouts, measurements, colors, assets, or key
bindings were copied or translated.
