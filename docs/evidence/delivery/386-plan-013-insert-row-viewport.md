# Plan 013 residual — staged insert rows in VirtualGrid viewport

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `insert_row_display` pure helper (`+` glyph) | done |
| VirtualGrid foot paints staged inserts | done |
| Synthetic abs keys avoid resident collision | done |
| Totals include insert count | done |
| Empty values show `∅` | done |
| Unit test | done |

## Decision

Insert drafts are not resident server rows. Paint them as a footer band with
`+` markers so operators see pending inserts without inventing fake abs
rows in the model. Edit still goes through EditInsert / Review; cursor
navigation into synthetic rows remains optional.

## Evidence

```text
cargo test -p tablerock-tui --lib insert_row_display
cargo test -p tablerock-tui --lib
```

## Remaining work

- Cursor focus into synthetic insert rows (optional)
