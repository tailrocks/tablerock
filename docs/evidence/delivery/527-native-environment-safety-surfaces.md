# Native environment and safety surfaces

Date: 2026-07-19

## Behavior

Saved-profile environment and safety facts now remain visible across every
required native surface: connection row, profile editor, workbench context,
and query/object tabs. Production uses an explicit warning icon and text;
read-only uses a lock and text. Accessibility labels state both facts, so
meaning never depends on color.

Swift performs presentation-only label mapping. Environment, production
warning, and safety mode remain Rust-owned profile facts.

## Evidence

- Native release build: pass.
- Profile/editor/context/tab structural assertions: pass.
- Runtime fixture with active production/confirm-writes profile: pass.
- Existing profile group, health, reconnect, ordering, and empty-group gate:
  pass.

## Provenance

TablePro was used only to confirm broad environment and safety marker concepts.
No source, tests, text, screenshots, layouts, measurements, colors, assets, or
key bindings were copied or translated.
