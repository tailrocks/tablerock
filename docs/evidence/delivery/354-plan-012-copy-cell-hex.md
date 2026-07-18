# Plan 012 residual — copy cursor cell (text / hex)

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `format_cursor_cell` raw text; NULL → empty | done |
| `format_cursor_cell_hex` lowercase hex of UTF-8 bytes | done |
| Actions CopyCell / CopyCellHex → OSC 52 | done |
| Pending cell fail closed | done |
| Unit test | done |

## Decision

Cell copy is independent of multi-column result copy formats. NULL becomes
empty string (not the word NULL) so paste into prompts is clean. Hex is
presentation UTF-8 of the cell text (not re-decoded binary arena).

## Evidence

```text
cargo test -p tablerock-tui --lib format_cursor_cell
cargo check -p tablerock-tui
```

## Remaining work

- Copy row via CopyScope::Row action shortcut (optional)
