# Native macOS Experience

The macOS app is a native projection of the same product spec screens over the
same Rust core. It is not a terminal in a window and not a second product:
Swift owns presentation and OS integration; Rust owns every database, safety,
and state decision ([native path](../architecture/native-macos-path.md)).

Design target: **macOS 26 Tahoe and later**, with the **Liquid Glass** design
system. macOS 27 removes the Liquid Glass compatibility opt-out, so the app is
built glass-native from the start; there is no legacy-material mode to carry.

## Design language

Liquid Glass is a dynamic translucent material for the **navigation and
control layer that floats above content**. Applied to this product:

| Surface | Treatment |
|---|---|
| Window toolbar (context bar) | Glass. Connection, database/schema pickers, safety badge, run/cancel, save-with-pending-count, preview, export |
| Sidebar catalog | Glass sidebar; content extends under it edge-to-edge |
| Sheets, popovers, completion, review dialog | Glass (transient UI) |
| Data grid, SQL editor, text content | **Never glass** — opaque/system backgrounds; content scrolls *under* the glass layer |
| Status bar | Non-interactive items without glass (`isBordered = false` / `sharedBackgroundVisibility(.hidden)`) |

Rules this app follows (from Apple's guidance):

- Content never sits *in* glass; it scrolls *under* it. Result grids use a
  hard scroll-edge effect for a clean cutoff under the toolbar.
- One glass cluster per region: toolbar items grouped with `ToolbarSpacer`;
  custom glass elements grouped in one `GlassEffectContainer` per cluster.
  Never stack glass on glass, never scatter small glass surfaces.
- Tint only primary meaningful actions (Run, Apply changes) with
  `.regular.tint`; decoration stays untinted.
- No custom blurs, no `NSVisualEffectView` sidebar material, no custom toolbar
  backgrounds — they fight the system edge effects.
- Environment (production) marking is a label plus accent treatment on the
  toolbar badge, independent of glass translucency; critical state is never
  conveyed by translucency or color alone.
- Accessibility is structural: Reduce Transparency, Increase Contrast, and
  Reduce Motion degrade the material gracefully — behavior and legibility
  never depend on it.

## App structure

- `App` with `WindowGroup` (one window per connection session; multi-window
  supported), `Commands` menus, and a `Settings` scene. Commands route through
  `@FocusedValue` so Run/Cancel/Save act on the focused tab.
- `NavigationSplitView`: sidebar catalog (glass) + content. Content extends
  under the sidebar; inspector as trailing column or accessory.
- Data-dense controls stay AppKit: `NSOutlineView` (catalog), `NSTableView`
  (grid), `NSTextView`/TextKit (SQL editor) via `NSViewRepresentable`.
  SwiftUI `Table`/`List` do not replace them at workbench scale.
- Toolbar is user-customizable (`.toolbar(id:)`), with `ToolbarSpacer`
  separating connection, editing, and action clusters.
- Dense professional layout preserved with `controlSize(.small)` /
  `prefersCompactControlSizeMetrics` in inspectors and accessory bars.
- Window tabs follow native `NSWindowTabbing`; the preview-tab and
  per-tab-state rules in [Workbench](workbench.md) apply unchanged.
- SF Symbols with restrained symbol effects for run/status feedback; never
  indefinite effects inside grid rows.

## Swift ownership (unchanged, restated for design work)

- `@Observable` classes on `@MainActor` hold immutable Rust snapshots; no
  `ObservableObject` in new code.
- An actor-owned UniFFI bridge client polls bounded event/page batches off the
  main actor and publishes immutable projections.
- Swift 6 strict concurrency throughout; no GCD.
- Swift contains no database, parser, mutation, history, redaction, or safety
  behavior. Presentation-only formatting (dates, sizes) is allowed; anything
  semantic arrives as a Rust-owned typed fact.

## Screen mapping

Every screen in this specification maps to native idioms as listed in its own
"Both clients" table: connection list → SwiftUI list with sections and
search; editor → SwiftUI form; catalog → outline view; grid → table view;
review → sheet; completion → native popup; password prompt → secure field;
export → `NSSavePanel`. The [connections](connections.md) screen adds the
native-only Keychain password source.

The trailing value inspector shows column/type/nullability/truncation facts,
display text, and raw bytes as hexadecimal. Structured JSON values also expose
a deterministic key-sorted tree. Tree decoding fails closed above 64 KiB,
1,024 nodes, or 64 levels; malformed/non-JSON structured values keep the text
and hex views without presenting a false tree.

Table-like object tabs expose ordered server sorting, typed parameterized
filters, and an explicitly labeled raw-WHERE editor. Raw fragments are capped
at 64 KiB and pass unchanged to Rust for validation and parenthesized query
composition; Swift never constructs SQL. Apply and clear are explicit actions,
and active raw mode remains visibly announced. Result column headers project
each active server sort's direction and one-based priority; unsorted headers
retain the database column name unchanged.

## Platform evidence gates

The native design is accepted only with the phase 12-14 gates: strict
concurrency build, VoiceOver/keyboard/IME coverage, multi-window restoration,
Instruments page/scroll performance, clean-machine signing/notarization, and
Liquid Glass behavior verified across light/dark, Increase Contrast, and
Reduce Transparency.
