# Plan 021 — native query tabs

Date: 2026-07-19

## State ownership

Each native query tab owns statement text, decoded result snapshot, result ID
and revision, next-page cursor, running operation ID, cancel/write/review
outcomes, query errors, and SQL-file binding. Async execute, cancel, pagination,
and review methods capture the originating tab, so selecting another tab while
an operation awaits cannot redirect its completion into the visible tab.

The strip supports add, select, rename, and close with a 64-tab ceiling. At
least one tab remains. Running tabs cannot close; dirty tabs require explicit
discard confirmation. Query summary/error state is tab-owned rather than
sharing catalog status.

## Restoration and connection boundaries

Saved-profile edits debounce into the typed Rust session-intent bridge. Profile
connect restores database intent, titles, text, and selected index. A profile
without intent starts one empty tab. Result pages, pagination handles,
operation state, review outcomes, and SQL-file bindings never restore.

Connection replacement is rejected while any query tab runs. Successful
replacement clears volatile state across every existing tab before restoring
target-profile intent. This prevents old-session results from appearing under
a new connection.

## Evidence

| Gate | Result |
|---|---|
| two-tab text/result/running/summary isolation fixture | pass |
| add/rename/close structural contract | pass |
| dirty/running/last-tab close guards | pass |
| 64-tab bound in Swift and Rust | pass |
| typed intent persistence conformance | pass; UniFFI 19 tests, 5 ignored |
| strict Swift 6 release build | pass |
| SQL-file/saved-query/history/accessibility native regressions | pass |

## Remaining boundary

Object preview/pinned tabs, multiple result sections per statement, independent
native windows, and full process crash/relaunch restoration remain. This
checkpoint does not claim them.

## Provenance

TablePro was used only to confirm broad independent-tab and intent-restoration
concepts. No source, tests, text, screenshots, layouts, measurements, colors,
assets, or key bindings were copied or translated.
