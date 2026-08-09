# Signature interaction research: what makes TableRock memorable

Date: 2026-08-09  
Kind: product-design research (clean-room). Competitors establish **workflow
existence** only—no layouts, colors, geometry, product strings, or source.

---

## 1. Product truth (TableRock)

- **Rust** owns connections, catalogs, queries, pages, editability, staged
  mutations, review SQL, safety, history, redaction.
- **Swift** owns windows, toolbar, NavigationSplitView, AppKit grid/catalog/
  editor, presentation stores.
- Engines: PostgreSQL, ClickHouse, Redis (plus local sample SQLite).
- Spec already names **Change Ledger**, **Environment Halo**, **Relation Lens**
  as product grammar; staging/review/FK edges exist as *truth* but not yet as
  one unforgettable continuous interaction.

Luxury here means **precision, continuity, and trust**—not decoration.

---

## 2. Category: inherited assumptions (discovered)

Across table-first, IDE-first, and schema-first clients, the field largely
inherits the same mental model:

| Inherited assumption | Consequence |
|---|---|
| **Catalog tree is the map of the world** | Schema becomes a filing cabinet; relationships are afterthoughts |
| **Table is the primary object** | Rows lose “where they live in the graph” |
| **Related data = open another tab/window** | Spatial context is destroyed (TablePlus-class FK often opens a tab) |
| **ER diagram is a separate mode** | Diagrams are “visited,” not used mid-edit |
| **SQL editor is a separate mode** | GUI and SQL fight instead of trading |
| **Confirm is a generic dialog** | Production risk feels the same as a local renames |
| **History is a flat list of strings** | No structural or causal story |
| **AI is a chat dock** | Detached from grid selection and ledger |
| **Every engine looks like tables** | Redis/ClickHouse force-fit into PG metaphors |
| **Density = lots of equal chrome** | Hierarchy collapses into pill rows |

What users praise (recurring **sentiment**, not stats): speed, native feel,
inline edit, low ceremony.  
What they complain about: bloat, slow IDEs, weak relationship UX, subtle
production danger, “another tab” for every detour.

**Whitespace:** *stay in the grid plane* while consequence and relationships
become first-class **spatial** objects—not separate apps inside the app.

---

## 3. Award / polish principles (not looks)

From Apple Design Awards messaging (2024–2026) and platform design talks:

- **One clear idea** beats a feature catalog (*grug*: singular concept).
- **Comprehensible data** (*Tide Guide*: charts you understand in a glance).
- **Progressive disclosure** and focus (Interaction winners; iA Writer / Mela
  class: dim what is not the task).
- **Hierarchy**: what matters is obviously primary (WWDC design principles).
- **Accessibility is product**, not checklist.

Transfer to DB client: the memorable moment must happen **during normal
exploration**, dozens of times a day—not in a tutorial or a separate “wow”
screen.

---

## 4. Ten substantially different concepts

Scoring axes (1–10): U usefulness · F frequency · D differentiation · V visual
potential · I interaction · N macOS nativeness · Feas feasibility · A a11y ·
P expert productivity. **Total = sum**.

### C1 — Row Continuum (spatial relation peek)

**Problem:** Following a foreign key usually opens a new tab and breaks
context.  
**Existing:** New tab / popover / separate ER.  
**Interaction:** On an FK-bearing cell, Continuum opens a **peer plane** beside
the grid showing related rows for that value; Escape restores.  
**Spatial:** Grid stays; continuum panel originates from selection; unrelated
chrome quiets.  
**15–30s:** Click `album_id=1` → Continuum → see album + artist facts → open
parent or stage a change on child → Escape.  
**Screenshot identity:** Split plane “from this cell → these rows,” not a tree.  
**Frequency:** High whenever FKs exist.  
**Native fit:** `HSplitView`, focus rings, keyboard, dimming.  
**A11y:** VO “Relation Continuum, outbound to albums, 1 row”; ⌘⌥→ open; Esc.  
**Architecture:** Presentation can fixture edges + preview pages; production
needs Rust “neighbors for (schema,table,column,value)” page contract.  
**Danger:** Becomes ER gimmick if it shows the whole graph.  
**Scores:** U9 F9 D9 V9 I9 N9 Feas8 A8 P9 → **79**

### C2 — Consequence Stage (ledger as first-class plane)

**Problem:** Staged edits are easy to forget until apply.  
**Existing:** Status counts, review dialog, dirty tabs.  
**Interaction:** Ledger rail expands from status; each entry shows before/after
and exact SQL preview; Apply is on the rail.  
**Spatial:** Bottom/side plane; grid marks stay.  
**Identity:** “Pending mutations look like a score.”  
**Frequency:** High for writers.  
**Native fit:** Sheets + glass chrome; not web modals.  
**Architecture:** Already mostly Rust-owned drafts.  
**Danger:** Second grid competes with data.  
**Scores:** U9 F8 D7 V7 I8 N8 Feas9 A9 P9 → **74**

### C3 — Production Gate Frame

**Problem:** Production and local look the same until disaster.  
**Existing:** Colors/badges; easy to miss.  
**Interaction:** Connected production profiles get a persistent **frame**
(title bar accessory + non-color HALO) and write actions re-label
“Apply on PRODUCTION”.  
**Frequency:** Medium-high for ops.  
**Danger:** Alarm fatigue if too loud.  
**Scores:** U8 F7 D7 V6 I6 N9 Feas9 A9 P8 → **69**

### C4 — Explain Path Ribbon

**Problem:** Query cost is a separate EXPLAIN screen.  
**Existing:** Explain tabs/panels.  
**Interaction:** After run, a collapsible ribbon under the editor shows plan
summary facts (seq scan / index / cost class) owned by Rust.  
**Frequency:** Medium.  
**Danger:** Fake precision / noise.  
**Scores:** U7 F6 D6 V7 I6 N7 Feas6 A7 P7 → **59**

### C5 — Semantic Cell Typography

**Problem:** UUID, money, JSON, FK all look like strings.  
**Existing:** Some type coloring (often color-only).  
**Interaction:** Glyph+typeface rules per distinction (already partially
spec’d); Continuum affordance on FK glyphs.  
**Frequency:** Continuous.  
**Danger:** Busy grid.  
**Scores:** U7 F10 D5 V6 I5 N8 Feas8 A8 P7 → **64**

### C6 — Multi-model Workbench Dial (PG / CH / Redis)

**Problem:** Three models forced into table metaphor.  
**Existing:** Mode switches / different panels.  
**Interaction:** Context dial morphs catalog + empty states per engine
capability without changing shell.  
**Frequency:** High for multi-engine users.  
**Danger:** Three half-products.  
**Scores:** U8 F7 D7 V5 I6 N7 Feas5 A7 P7 → **59**

### C7 — History as Causal Timeline

**Problem:** History is a string list.  
**Existing:** Searchable statements.  
**Interaction:** Timeline nodes for run / apply / fail with links back to
ledger snapshots.  
**Frequency:** Medium.  
**Danger:** Heavy UI for little gain.  
**Scores:** U6 F5 D6 V6 I6 N7 Feas5 A7 P6 → **54**

### C8 — Selection → SQL Round-trip

**Problem:** GUI filter and SQL diverge.  
**Existing:** “Copy as SQL” fragments.  
**Interaction:** Current browse plan always editable as structured chips **and**
as SQL projection from Rust; edits re-validate.  
**Frequency:** High for power users.  
**Danger:** Broken two-way sync.  
**Scores:** U8 F7 D6 V4 I7 N7 Feas5 A7 P8 → **59**

### C9 — Data Quality Spotlight

**Problem:** Null/empty/outlier density is invisible.  
**Existing:** Separate profilers.  
**Interaction:** Column header spark/null% from Rust profile facts; click
filters to those rows.  
**Frequency:** Medium.  
**Danger:** Dashboard clutter.  
**Scores:** U7 F5 D6 V7 I6 N7 Feas5 A7 P6 → **56**

### C10 — AI as Proposal, not Chat

**Problem:** Chatbots detach from grid.  
**Existing:** Side chat.  
**Interaction:** AI only emits **ledger proposals** and Continuum hints bound
to selection; no free chat dock.  
**Frequency:** Medium.  
**Danger:** Trust / wrong SQL.  
**Scores:** U6 F5 D8 V5 I7 N6 Feas3 A6 P5 → **51**

### C11 — Compare Twin Tabs Spatially

**Problem:** Comparing filters needs two windows.  
**Existing:** Multiple tabs, mental A/B.  
**Interaction:** Split twin of same object with linked scroll option.  
**Frequency:** Medium-low.  
**Scores:** U6 F4 D5 V6 I6 N8 Feas7 A7 P6 → **55**

### C12 — Disconnect / Stale Reality Fog

**Problem:** Stale pages look live.  
**Existing:** Banners.  
**Interaction:** Content desaturates + “STALE / DISCONNECTED” word mark;
actions freeze with reason.  
**Frequency:** Medium.  
**Scores:** U7 F5 D4 V5 I5 N8 Feas9 A9 P7 → **59**

---

## 5. Top three (deep)

### #1 Row Continuum (C1) — **flagship**

**Why it wins:** Highest total; high frequency; visual/interaction identity;
fits native split views; improves expert productivity without leaving the
grid plane; differentiates from “open related tab.”

**Rust contract (future):**  
`relation_neighbors(session, from:{schema,table,column}, value, direction,
limits) → page of related rows + edge identity`.  
No SQL construction in Swift.

**Fixture phase:** Deterministic neighbor tables for sample schema
(`tracks.album_id` → albums, `albums.artist_id` → artists) and PG demo edges.

### #2 Consequence Stage / Change Ledger plane (C2)

**Why:** Aligns with existing mutation truth; makes TableRock famous for
*safe* editing. Slightly less “one-screenshot unique” than Continuum.

### #3 Production Gate Frame (C3)

**Why:** Trust. Less continuous delight; essential safety grammar (Halo).

---

## 6. Flagship flow: Row Continuum (detail)

### Resting
Grid + optional value inspector. No continuum.

### Hover / focus (FK cell)
Cell shows focus ring; status hint: `Continuum · ⌘⌥→` (text, not color alone).

### Open (⌘⌥→ or toolbar **Continuum**)
1. Selection must include a column known to participate in a relation edge
   (fixture or Rust).
2. Continuum plane appears **to the trailing side of the grid** (not a modal).
3. Title: `CONTINUUM · tracks.album_id → albums.id` + direction word.
4. Related rows load (fixture or page).
5. Unrelated inspector content can compress; grid remains interactive.

### Loading
Progress label `Loading related rows…` in continuum; grid not blocked.

### Empty
`No related rows for this value` + edge identity still shown.

### Error
`Continuum unavailable: {redacted reason}` + Retry; grid unchanged.

### Stale / disconnected
Continuum shows `STALE` / `DISCONNECTED` word; Open disabled with reason.

### Production
Halo remains visible; Continuum does not hide production state.

### Destructive
Continuum is read-first; staging edits still go through ledger.

### Narrow window
Continuum becomes a sheet from the bottom/trailing edge; Esc still dismisses.

### Keyboard
- `⌘⌥→` open/refresh continuum for selection  
- `⌘⌥←` or `Esc` close  
- Continuum grid: arrow keys; `⌘⌥↓` open nested hop (v2)  
- VoiceOver: announces edge, row count, and how to dismiss  

### Reduce Motion
Plane appears without slide; cross-fade 0 or instant swap.

### Reduce Transparency / Increase Contrast
Continuum uses opaque content background (never glass content); chrome may
use system glass.

### Escape
Closes continuum; focus returns to originating cell.

---

## 7. Ideas that sound impressive but should **not** lead

| Idea | Why not flagship |
|---|---|
| Full-screen 3D schema galaxy | Novelty; low daily use; a11y nightmare |
| AI chat as home surface | Detaches from data; trust issues |
| Web-style dashboard home | Anti-native; anti-expert density |
| Glass everywhere | Hierarchy death; HIG anti-pattern |
| Animated mascot onboarding | Not for multi-hour professional tools |

---

## 8. Final decision

> **If TableRock wants one design idea to become famous for, it should be
> Row Continuum — spatial relationship navigation that never abandons the
> grid plane — because every other serious client still solves “related data”
> by breaking context, and TableRock already has the safety/ledger spine to
> make staying in context *safe*, not just pretty.**

---

## 9. Prototype status (this delivery)

- Presentation-only **Row Continuum** panel in native result layout
  (`relation.continuum.*` accessibility ids).
- Fixture neighbor rows for sample-like columns (`album_id`, `artist_id`).
- **⌘⌥→** open, **⌘⌥←** / Close dismiss; plane replaces value inspector while open.
- Clearly labeled **FIXTURE** until Rust `relation_neighbors` ships.
- No database truth invented beyond deterministic fixture tables.
- `swift build --target TableRockApp` succeeds.

### Self-critique (iteration 1)

| Observation | Response |
|---|---|
| Fixture badge might be ignored | Keep high-contrast FIXTURE word; never omit |
| Continuum vs full relationships sheet | Continuum is hop-from-cell; sheet remains graph inventory |
| Nested hops not in v1 | Correct — depth-1 only until contract exists |
| Empty selection path | Button disabled + error string; no silent no-op |

### Rust contract sketch (not implemented)

```text
relation_neighbors(
  session_id,
  from: { schema, table, column },
  value: OwnedValue,
  direction: outbound | inbound,
  limits: PageLimits
) -> ResultPage + edge_identity
```

Swift must not build SQL for neighbors.
