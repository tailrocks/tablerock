# Component and material map

## Shared components

| Component | Purpose | Native basis | Material |
|---|---|---|---|
| Context toolbar | Connection/database context and global actions | SwiftUI toolbar-style controls | Regular glass at the top-level container |
| Catalog sidebar | Search and hierarchical navigation | Sidebar list/outline conventions | System sidebar material; opaque fallback under Reduce Transparency |
| Tab strip | Object/query ownership and close state | Native tab/document conventions | Opaque underlay with restrained selected fill |
| Data grid | Headers, rows, selection, reorder and resize affordances | Narrow `NSViewRepresentable` boundary around `NSTableView` | Opaque content background only |
| SQL editor | Monospaced input, line numbers, selection, completion anchor | Native text-editor semantics | Opaque content background only |
| Result pane | Execution state, messages, result tabs, data | Split-view content | Opaque content background only |
| Value inspector | Typed value detail and copy affordances | Inspector conventions | Opaque or system background; never decorative glass |
| Change ledger | Pending operations and risk summary | Inspector/list conventions | Opaque content; glass only on detached controls |
| Sheet/form | Setup and scoped confirmation | Native Form/sheet conventions | System sheet background |
| Floating palette | Canvas-only navigation/action cluster | Top-level custom control | Regular interactive glass |

## Liquid Glass rules

1. Prefer automatic/system material from native toolbar, sidebar, sheet, menu,
   and control styles.
2. Apply custom `glassEffect` only to a top-level navigation or command
   container. Apply it after shape and appearance modifiers.
3. Group nearby custom glass controls in `GlassEffectContainer`; do not put
   every button in its own effect.
4. Use regular glass. Clear glass is disallowed because database workbench
   backdrops are not media-rich and cannot guarantee legibility.
5. Grid, editor, result, form, inspector, and review bodies remain opaque.
6. Under Reduce Transparency, replace custom glass with an opaque semantic
   background and explicit separator stroke.
7. Under Increase Contrast, strengthen separators and selected/focus outlines.
8. Under Reduce Motion, disable decorative transitions and glass morphing.

## Semantic tokens

TableRock uses system colors and fonts plus role names—not copied product tokens:

| Role | Contract |
|---|---|
| Canvas | `windowBackgroundColor` |
| Content | `controlBackgroundColor` or `textBackgroundColor` |
| Raised content | `underPageBackgroundColor` with separator |
| Selection | system accent with label-preserving foreground |
| Primary text | `labelColor` |
| Secondary text | `secondaryLabelColor` |
| Separator | `separatorColor`, strengthened for contrast preview |
| Safe/read-only | icon + text + semantic system tint |
| Risk/production | icon + explicit word + semantic system tint |
| Data type | abbreviated text label; color is supplemental |

Default density targets 28-point grid rows, 32-point compact tool rows, and
44-point primary actions. Resizing may expose more content but must not remove
the command hierarchy.
