# Design system + hierarchy pass

Date: 2026-08-09

## What

1. Added [`docs/product/design-system.md`](../../product/design-system.md):
   philosophy, competitive matrix (workflow-only), ADA-aligned principles,
   surface/control/type rules, ranked P0–P3 backlog, TUI transfer.
2. Native hierarchy fix: **tab strip no longer paints glass on every tab**—
   selected uses `.glassProminent`, unselected `.plain`, add uses `.glass`.
3. Result tool strip wrapped in one `GlassEffectContainer`; denser meta copy.
4. Profile rows show **HALO** environment words without color-only meaning.

## Why

Award-class apps reward one coherent idea and strong hierarchy. Prior glass
adoption improved materials but risked “pill overload” (same critique as
generic glass UIs). Content-dominates-chrome requires restraint.

## Clean-room

Principles from public award/HIG/Liquid Glass guidance and category workflow
existence. No competitor layouts, colors, assets, or product strings.

## Verification

```text
rg -n 'buttonStyle\(\.glass\)' native/Sources/TableRockApp/TableRockApp.swift | rg 'tab'
# selected tabs: glassProminent only in QueryTabStrip helpers
cargo test -p tablerock-tui --test craft_hierarchy
cargo test -p tablerock-ffi --test sample_sqlite
```

## Remaining debt (next three)

1. Change Ledger native affordance when staged count &gt; 0.
2. Relation Lens from grid cell selection (≤2 actions).
3. Guided sample path: Sample → Lens → stage → ledger → revert.
