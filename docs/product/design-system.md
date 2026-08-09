# TableRock Design System (native + TUI principles)

Authoritative design direction for the TableRock workbench. Complements
[`native-macos.md`](native-macos.md) (Liquid Glass rules) and
[`workbench.md`](workbench.md) (layout). Evidence: 660–664.

Clean-room: competitors establish *that* workflows exist; they do not dictate
geometry, color, copy, or interaction sequences.

---

## 1. Design philosophy (five principles)

1. **Quiet precision** — Luxury is spacing, type, latency, and trust—not
   decoration. Prefer system materials over custom chrome.
2. **Content dominates chrome** — The grid, editor, and catalog are the
   product. Glass lives only on navigation/control layers.
3. **Consequence before mutation** — The Change Ledger makes every write
   visible and reversible before apply. Production is never quiet.
4. **Context before navigation** — Relation Lens surfaces how the selection
   connects to the rest of the database without leaving the workbench.
5. **Progressive complexity** — Default density is expert-ready; advanced
   tools appear as lenses, sheets, and inspectors—not permanent panels.

**Product story (award narrative, not a claim):** *A native data instrument
that makes relationships spatial and dangerous changes calm, visible, and
reversible.*

---

## 2. Signature interactions

| Signature | Operator job | Primary surface |
|---|---|---|
| **Environment Halo** | Know where you are (dev / staging / production) without color alone | Toolbar badge, context bar, profile rows |
| **Change Ledger** | See staged inserts/updates/deletes before apply | Status `ledger N`, review sheet, TUI **Ledger** |
| **Relation Lens / Row Continuum** | Traverse FK / neighborhood **without leaving the grid plane** | Continuum peer plane (⌘⌥→), relationships sheet, TUI **Lens** |

Rank: Halo + Ledger ship continuously; Lens deepens as FK facts are available.

---

## 3. Competitive matrix (workflow existence only)

| Family | Strength to retain | Weakness to beat |
|---|---|---|
| Table-first native (TablePlus/TablePro class) | Direct grid, low ceremony, native feel | Relationships secondary; production risk subtle |
| IDE-first (DataGrip/DBeaver) | Depth, intelligence | Density, intimidation, weak identity |
| Schema-first (Azimutt/ChartDB class) | Relationship neighborhoods | Separate from daily edit/query |
| Friendly/AI-first | Approachability | Web feel; AI as detached panel |

**Whitespace:** context-and-consequence-first, still dense and native.

---

## 4. Apple Design Award principles applied

| Principle (from awards / WWDC design guidance) | TableRock application |
|---|---|
| One clear idea | Data Observatory: context + consequence |
| Hierarchy & clarity | Glass only on chrome; content opaque; one primary action per cluster |
| Progressive disclosure | Sheets/lenses vs always-on panels |
| Comprehensible data | Typed cell distinctions; non-color status |
| Accessibility in the concept | Halo text, ledger counts, VoiceOver labels, Reduce Transparency |
| Platform integration | Tahoe glass, customizable toolbar, NavigationSplitView, AppKit dense controls |

---

## 5. Design system tokens (semantic, not palette kits)

### Surfaces

| Layer | Treatment |
|---|---|
| Window chrome / toolbar / sidebar | System Liquid Glass |
| Transient (sheet, popover, menu) | System glass / standard sheet |
| Content (grid, SQL editor, list bodies) | Opaque `textBackgroundColor` / system content |
| Halo / ledger badges | Small glass capsule **or** plain semantic text—never full-bleed glass content |

### Controls

| Role | Style |
|---|---|
| Primary action (Run, Sample, Apply, Connect) | `.glassProminent` (sparingly) |
| Secondary chrome action | `.glass` inside one `GlassEffectContainer` |
| Dense lists / rows | `.plain` |
| Tabs (workbench) | **Not every tab is glass** — selected: weight + subtle glass; unselected: plain; only one primary accent |
| Destructive | System destructive role + confirm sheet |

### Typography

| Use | Guidance |
|---|---|
| Window / screen titles | `.title` / `.title2` semibold, slight tracking |
| Section labels | `.headline` or `.subheadline` |
| Status / meta | `.caption` / `.caption2`, secondary |
| SQL / identifiers / numeric cells | Monospaced where semantic (AppKit/editor) |
| Environment Halo word | Caption **bold** + uppercase word PRODUCTION/STAGING |

### Density

- Default control size for workbench chrome: `.small` / compact metrics.
- Grid row height: AppKit dense default; do not inflate for “luxury.”
- Spacing in tool strips: 6–10 pt; content stacks: 6–12 pt.

### Color & semantics

- **Never** convey production, dirty, or error by color alone.
- Production: text `HALO PRODUCTION` + warning symbol + safety line.
- Ledger: text count `ledger N` (TUI) / “Change Ledger” in review.
- Pending/dirty tabs: glyph `*` or pin, not red alone.

### Motion

- Prefer system transitions; no indefinite symbol effects in grid rows.
- Respect Reduce Motion (instant state swap).
- Respect Reduce Transparency / Increase Contrast (legible without glass).

### Forbidden

- Glass on grids/editors/scroll content.
- Custom blur stacks fighting system edge effects.
- Glass-on-glass piles of pill buttons for every tab and tool.
- Dashboard cards, giant empty states for experts, AI-first chat docks.
- WebView or CSS design-system ports.

---

## 6. Screen audit summary (current)

| Screen | Strength | Gap |
|---|---|---|
| Connections | Search, groups, Sample CTA | Action strip density OK; list still generic |
| Workbench shell | Split + toolbar | Detail stack can feel like a form dump |
| Tabs | Functional | **Glass on every tab flattens hierarchy** |
| Grid + inspector | AppKit dense | Tool strip can look bolt-on |
| Halo | Non-color text | Must stay on every connected surface |
| Ledger | Rust staging truth | Presentation naming still maturing |
| Relation Lens | PG relationships sheet | Not yet continuous from cell selection |
| SQL editor | NSTextView | Action cluster improved with glass container |
| Empty/error | ContentUnavailable | Sample path strong |

---

## 7. Prioritized improvements

### P0 — foundational quality

1. Reduce glass-pill tab strip; restore hierarchy.
2. One glass cluster per tool region; never scatter.
3. Halo visible whenever session is connected.
4. Opaque content surfaces verified (grep gate).

### P1 — interaction

1. Change Ledger affordance when staged count &gt; 0 (native + TUI).
2. Relation Lens from selection in ≤2 actions when edges exist.
3. Denser status line: rows · timing · ledger · focus hints.

### P2 — signature experiences

1. Guided sample: Sample DB → Lens → stage → ledger → revert.
2. Spatial continuity: table → related table without losing context.
3. Semantic cell treatment already in product—amplify without noise.

### P3 — polish

1. Micro-transitions on tab select / sheet present.
2. Icon multilayer lens metaphor (release asset).
3. Instruments pass for scroll/page latency.

---

## 8. TUI transfer

Terminal cannot use Liquid Glass. Transfer:

- Halo / Ledger / Lens **words** in status and actions.
- Sparse hierarchy; non-color cues; restrained borders.
- Same product sequence: connect → catalog → query → ledger → apply.
