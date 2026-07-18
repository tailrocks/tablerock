# Plan 021 — native SQL files

Date: 2026-07-19

## Rust contract

UniFFI exposes coarse SQL-file read and write operations. Rust requires an
absolute `.sql` path capped at 4,096 bytes, strict UTF-8 text capped at 8 MiB,
and returns path, length, and modification-time facts. Bound-file saves compare
the observed facts before writing and reject external changes unless the user
explicitly authorizes overwrite.

Atomic replacement writes and syncs a sibling temporary file, then renames it
over the destination. The prior helper pre-deleted existing destinations,
creating a data-loss window before rename; that enabling condition is removed.
A failed rename cleans the temporary file while preserving the destination.

## Native behavior

AppKit `NSOpenPanel` and `NSSavePanel` select SQL URLs. Open warns before
discarding editor changes. Save As supplies a `.sql` extension. Bound saves
carry Rust-issued file facts; an external conflict offers Reload External
Changes, Overwrite External Changes, or Cancel. Reload and overwrite wording
states which editor or external text is discarded.

Every successful `startAccessingSecurityScopedResource()` call is paired with
`stopAccessingSecurityScopedResource()` immediately after the Rust operation.
This follows Apple's requirement to balance security-scoped access and release
it promptly.

## Evidence

| Gate | Result |
|---|---|
| persistence SQL-file suite | pass; 4 focused, 38 full |
| UniFFI atomic/external-conflict conformance | pass; 19 tests, 5 ignored |
| native SQL-file structural/runtime fixture | pass |
| native saved-query/history/accessibility regressions | pass |
| strict Swift 6 release build | pass |

## Documentation source

Context7 resolution was attempted first but its quota was exhausted. Current
API behavior was verified against Apple Developer Documentation for
[`NSSavePanel`](https://developer.apple.com/documentation/appkit/nssavepanel)
and
[`startAccessingSecurityScopedResource`](https://developer.apple.com/documentation/foundation/url/startaccessingsecurityscopedresource()).

## Provenance

TablePro was used only to confirm broad SQL-file open/save/reload and
external-change workflows. No source, tests, text, screenshots, layouts,
measurements, colors, assets, or key bindings were copied or translated.
