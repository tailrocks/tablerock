# Plan 009 residual — binary inspector multi-line hex dump

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| Hex panel: offset + 16-byte rows + ASCII gutter | done |
| Cap 256 bytes; “more bytes” marker | done |
| Inspector lines emit multi-line `hex:` block (max 20 lines) | done |
| Binary text panel notes shown count + CopyHex | done |
| Unit test | done |

## Decision

Dump uses presentation UTF-8 of `ProjectedCell::text` (same material as
CopyHex). Full binary arena re-decode stays engine-side; inspector stays
presentation-only.

## Evidence

```text
cargo test -p tablerock-tui --lib binary_hex
cargo test -p tablerock-tui --lib inspector
```

## Remaining work

- Page/scroll hex beyond 256 bytes — shipped as evidence 376
