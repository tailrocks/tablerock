# TermRock migration 0025 adoption

Date: 2026-07-17

## Decision

TableRock pins TermRock `main` revision
`0d85cfd17c6f00d7dc279cb6ad92f39e8d6c4f70` and adopts migration 0025 without a
compatibility table.

Before, CLI input routing matched keys directly while the TUI view separately
declared advertised hint strings. A binding could therefore change without its
hint changing. The earlier TermRock runtime-keymap work was only a spike.

Now the root `Model` owns one TermRock `Keymap<ShellKeyAction>`. Static defaults
use `KeyBinding::borrowed` and `Keymap::from_static`. CLI dispatch converts each
neutral key event to `KeyChord` and resolves it through that model-owned map.
The view clones the same map, disables context-inapplicable actions, and renders
TermRock `hint_spans`; it has no second chord or label table.

The public mutable keymap supports future settings-loaded remaps directly.
Remapping an action immediately changes both dispatch and its derived glyph;
tests prove the old chord becomes inert, the new chord dispatches, and the new
glyph renders. Context filtering is policy over the canonical map, not another
registry. A grouped arrow glyph advertises both action-selection directions,
while the right-arrow binding remains a hidden dispatch alias.

## Exact migration map

| Old TableRock approach | Current approach |
|---|---|
| direct `KeyCode` match in CLI | `Keymap::dispatch(KeyChord)` |
| product-local static hint arrays | `Keymap::hint_spans()` |
| immutable hardcoded shortcuts | model-owned `Keymap` with `remap`, `replace`, and `disable` |

## Evidence and provenance

- `cargo test -p tablerock-tui -p tablerock-cli --all-targets`
- runtime-remap dispatch and rendered-hint integration test
- complete TermRock migration 0025 and public API at the pinned revision
- no external product internals or protected expression were imported

Public source:
<https://github.com/tailrocks/termrock/blob/0d85cfd17c6f00d7dc279cb6ad92f39e8d6c4f70/migrations/0025-v0.11.0-runtime-configurable-keymaps.md>.
