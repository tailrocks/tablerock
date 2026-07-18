# Plan 009 residual — hex dump window paging

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `hex_source` + `hex_offset` on InspectorModel | done |
| 256-byte window with absolute offsets | done |
| Hex+ / Hex- page actions | done |
| Before/more markers on dump | done |
| Unit test | done |

## Decision

First window remains the default open view. Operators page with Hex+/Hex-
without reloading the cell. Presentation UTF-8 of the cell text is the
source (same as CopyHex); engine binary arena re-decode stays out of scope.

## Evidence

```text
cargo test -p tablerock-tui --lib hex_window
cargo test -p tablerock-tui --lib binary_hex
cargo test -p tablerock-tui --lib
```

## Remaining work

- None material for hex paging
