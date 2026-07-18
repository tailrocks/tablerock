# Plan 020 — Liquid Glass structural audit

Date: 2026-07-19
SDK: Xcode 26.6 / macOS 26.5 SDK

## SDK authority

Context7 was attempted first for current SwiftUI guidance but its monthly quota
was exhausted. The installed macOS 26.5 Swift interfaces were then inspected
directly. They confirm macOS 26 availability for `ToolbarSpacer`,
`sharedBackgroundVisibility`, `GlassButtonStyle`, `GlassProminentButtonStyle`,
`glassEffect`, and `GlassEffectContainer`.

## Audit and correction

The customizable toolbar used native toolbar chrome but lacked the required
cluster separators and used the older bordered-prominent style for Run. It now
uses fixed `ToolbarSpacer` boundaries between connection, catalog, and query
actions, and the sole primary toolbar action uses `.glassProminent`.

The AppKit result grid and SQL editor now explicitly set opaque system text
backgrounds on both document and scroll views. They cannot accidentally inherit
translucent content treatment. The system `NavigationSplitView` and toolbar own
their native navigation/control glass; no custom glass surface is layered over
them.

`verify-native-accessibility.sh` now enforces:

- at least two fixed toolbar cluster separators;
- glass-prominent primary toolbar action;
- opaque editor/grid content backgrounds;
- absence of `NSVisualEffectView`, blur, custom toolbar background, or custom
  material;
- existing semantic-label and ownership constraints.

Strict Swift 6 Release build and the expanded structural gate pass. Evidence
414 already supplies the light/dark × contrast × reduced-transparency matrix.
A fresh matrix attempt found and fixed a separate capture race: the helper had
selected the first layer-zero auxiliary window rather than the largest main
window. Pixel capture remains unavailable in the current host session, so no
new image claim replaces evidence 414.

## Provenance

TablePro supplied only the broad concept of a native database workbench toolbar
above opaque query content. No source, tests, text, screenshots, layouts,
measurements, colors, assets, or key bindings were copied or translated.
