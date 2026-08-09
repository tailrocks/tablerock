# Design research: Liquid Glass craft + sample-database onboarding

Date: 2026-08-09

## Purpose

Inform TableRock native Liquid Glass layering, connection/workbench simplicity,
and a **Try Sample Database** onboarding path. Clean-room: public workflow
*existence* and Apple HIG only — no competitor source, layouts, colors, key
bindings, assets, or product strings.

## Apple Liquid Glass (HIG / WWDC25)

Authoritative themes applied to TableRock:

1. **Navigation layer only** — Liquid Glass for toolbars, sidebars, and
   transient controls; **never** grids, editors, or scrollable content bodies.
2. **Content scrolls under chrome** — hard scroll-edge cutoff; content uses
   opaque/standard materials.
3. **No glass-on-glass** — one glass cluster per region; group toolbar items;
   avoid stacked translucent chrome.
4. **Tint sparingly** — glass-prominent / tint only on primary actions
   (Run, Apply, Try Sample primary CTA).
5. **Accessibility first** — Reduce Transparency / Increase Contrast /
   Reduce Motion must keep hierarchy readable without relying on glass alone.

Product mapping already lives in `docs/product/native-macos.md`; this
checkpoint tightens implementation to match those rules.

## Public competitor *workflow* themes (not UI copy)

| Source | Workflow existence only | TableRock response |
|---|---|---|
| TablePlus (marketing: native, simple, thoughtful UI) | Dense DB client can still feel light if chrome is minimal and native | Prefer system materials; fewer custom fills; clear empty states |
| TablePro (public docs: empty connections offer sample DB; Chinook-class demo) | Onboarding without server credentials | Bundled **TableRock-owned** SQLite sample + “Try Sample Database” (original schema/copy) |
| Superwhisper / CleanShot class (utility craft) | Focus, restraint, system materials, one primary action | Sparse connection empty state; one glass-prominent sample CTA |
| Apple Design Award principles (delight, interaction, visuals) | Hierarchy, harmony, consistency — not decoration | Layering + spacing + non-color status labels |

No TablePro/TablePlus/Jackin source or screenshots were read for layout
measurements.

## Craft principles TableRock adopts

1. **Simplicity** — empty connection list is an invitation, not a wall of
   chrome; sample open is one step.
2. **Native** — SF Symbols, system buttons, NavigationSplitView, glass toolbar.
3. **Hierarchy** — glass chrome / opaque content / quiet status text.
4. **Shared flow** — connect → catalog → query → result on TUI and native;
   native may be denser (glass, multi-window).
5. **Demo without network** — local SQLite file under operator data root.

## TUI craft themes

Terminal cannot use Liquid Glass; transfer hierarchy instead:

- Clear section titles and spacing on Connections empty/loading/error.
- Explicit non-color status lines (empty / sample ready / error).
- Same sample open entry near connection actions.
- Restrained borders; no palette explosion.

## Mapping to implementation

| Principle | Change |
|---|---|
| Glass on chrome only | Structural test + remove content-layer glass misuse |
| Sample onboarding | `Engine::Sqlite` local file + ensure fixture + Try Sample |
| Shared path | Rust authority for sample path + seed; native + TUI call it |
| TUI hierarchy | View layout constants + empty-state copy + tests |

## Remaining honesty

Subjective “award-winning” claims are out of scope. Gates are layering rules,
sample open proof, TUI hierarchy tests, and clean-room provenance.
