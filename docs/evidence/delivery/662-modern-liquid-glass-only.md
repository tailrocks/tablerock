# Modern Liquid Glass only (macOS 26 Tahoe)

Date: 2026-08-09

## Decision

TableRock native ships **Tahoe-first, no backward-compat chrome path**.

| Rule | Application |
|---|---|
| Platform floor | `macOS(.v26)` / `MACOSX_DEPLOYMENT_TARGET = 26.0` |
| Primary actions | `.buttonStyle(.glassProminent)` (Run, Sample, Connect, Apply) |
| Secondary chrome | `.buttonStyle(.glass)` in chrome clusters |
| Glass grouping | `GlassEffectContainer` for multi-button chrome strips |
| Forbidden | Custom `.background(.bar)` on chrome, `NSVisualEffectView` fills, glass on grid/editor content |
| Content | AppKit grid/editor stay `textBackgroundColor` (opaque) |

Inspiration: Apple WWDC25 Liquid Glass guidance (chrome/transient only,
group glass, tint sparingly) + product restraint (Superwhisper/CleanShot-class
focus). Clean-room: no competitor UI copy.

## Code map

- Sidebar inset: `GlassEffectContainer` + glass / glassProminent buttons
- Query action strip: same
- Empty-state primary CTA: glassProminent (+ large control size)
- Detail hierarchy: title/semibold + secondary status (non-color cues)
- Direct-connect form: SQLite engine option for local file paths

## Verification

```text
rg -n 'borderedProminent|background\(\.bar\)|NSVisualEffect' native/Sources
rg -n 'glassProminent|GlassEffectContainer|\.buttonStyle\(\.glass\)' native/Sources
rg -n 'textBackgroundColor' native/Sources/TableRockApp
```
