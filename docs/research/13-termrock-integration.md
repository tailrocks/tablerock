# TermRock Integration And TUI Architecture

## Decision

TermRock is TableRock's only reusable interactive TUI component layer.
TableRock uses Ratatui through TermRock's public contracts and composes
database-aware screens locally. It does not create duplicate focus, input,
panel, tab, list, dialog, scroll, form, grid, or editor primitives.

If TableRock needs a missing product-neutral primitive, that primitive is
designed, implemented, documented, tested, and published on TermRock `main`
first. TableRock then pins the reviewed full Git revision. Database concepts,
queries, values, mutations, and safety policy never move into TermRock.

## Evidence snapshot

Checked 2026-07-16:

- TermRock `main` at
  [`8cb3c88d`](https://github.com/tailrocks/termrock/commit/8cb3c88d118b2cbed10eef9d7cdbf0c0adbbbfde),
  version `0.6.0`, Rust 1.95 floor, Apache-2.0, Ratatui 0.30 component split,
  optional Crossterm 0.29 adapter, and published neutral `Tree`, `Form`, and
  `SplitPane`;
- Jackin at
  [`27c450e9`](https://github.com/jackin-project/jackin/commit/27c450e9af7a299171034f98267e0fa26bd3057f)
  pins the earlier compatible TermRock baseline `41482e9f` and uses Ratatui
  0.30; its workspace also compiles cleanly with a temporary exact pin to the
  `17974590` SplitPane evidence revision, proving the additive widgets preserve
  the existing consumer; TableRock imports no Jackin product internals;
- TermRock's [component inventory](https://github.com/tailrocks/termrock/blob/8cb3c88d118b2cbed10eef9d7cdbf0c0adbbbfde/crates/termrock/COMPONENTS.md),
  [migration boundary](https://github.com/tailrocks/termrock/blob/8cb3c88d118b2cbed10eef9d7cdbf0c0adbbbfde/MIGRATING.md),
  [interaction conventions](https://github.com/tailrocks/termrock/blob/8cb3c88d118b2cbed10eef9d7cdbf0c0adbbbfde/docs/content/docs/interaction.mdx),
  and [compatibility record](https://github.com/tailrocks/termrock/blob/8cb3c88d118b2cbed10eef9d7cdbf0c0adbbbfde/compatibility.toml)
  define the reusable boundary;
- Jackin's [TUI architecture](https://github.com/jackin-project/jackin/blob/27c450e9af7a299171034f98267e0fa26bd3057f/docs/content/docs/reference/tui/architecture.mdx)
  is the approved reference for Model/Message/Update/Effect/Subscription/View
  separation. TableRock copies no Jackin product code or model.

TermRock has no default features. TableRock enables its `crossterm` adapter,
uses Crossterm 0.29 `event-stream` in the CLI terminal adapter, and keeps the
TermRock/Ratatui/Crossterm versions lockstep-compatible. Use an exact Git
revision and commit `Cargo.lock`; do not depend on a branch.

## Existing TermRock surface

### Consume directly

| Need | TermRock contract | TableRock supplies |
|---|---|---|
| Semantic styling | `Theme` and style tokens | Product identity and database-status meaning |
| Framing | `Panel` | Titles, focus mapping, content |
| Top-level tabs | `Tabs` | Tab IDs, labels, dirty/running state |
| Selectable lists | `List` | Profiles, objects, commands, rows |
| Single-line input | `TextInput` | Validation, search/filter semantics |
| Actions | `ActionBar` | Capability-filtered actions and messages |
| Status | `StatusBar` | Safe summaries, progress, context |
| Hints | `HintBar` | Focus-aware primary bindings |
| Dialogs | `Dialog`, `ChoiceDialog`, `MessageDialog`, `Backdrop` | Lifecycle, wording, safety decisions |
| Notifications | `Toast` | Safe message and severity |
| Details | `DetailTable` | Profile/object/value metadata |
| Changes | `DiffView` | Redacted review projection |
| Scrollable text | `Viewport` and scroll helpers | Query error, DDL, raw values |
| Input routing | logical keymap/input types | Product actions and modal precedence |
| Interaction identity | stable focus/hover/hit regions | Product IDs and navigation policy |
| TEA runtime seam | `View`, `UpdateResult`, `Subscription`, frame driver | Root model, messages, effects, executor |
| Terminal lifecycle | Crossterm session adapter and typed OSC requests | Process policy and failure reporting |

TableRock may combine these primitives into product-local compositions such as
a connection card, Redis value screen, change review, or server overview.
Composition does not justify a second generic widget implementation.

TermRock also exports a trait named `Component`, but TableRock does not use it
to create component-owned state/handlers. The selected application pattern is
TEA; TermRock primitives receive root-model projections and emit semantic root
messages.

## Missing neutral primitives

These named primitives are required before the dependent TableRock screen
ships. Their public APIs follow TermRock's neutral contract.

| Primitive | Why neutral | Required contract | First TableRock dependency |
|---|---|---|---|
| `Tree` (published) | Hierarchical navigation recurs across products | Stable node IDs; disclosure state; depth; disabled/loading/error rows; keyboard/mouse; caller-owned lazy loading and filtering | Catalog and connection groups |
| `Form` / form layout (published) | Structured settings are not database-specific | Sections, labels, help/error text, required/disabled state, focus traversal, responsive one/two-column layout, caller validation | Connection editor and settings |
| `SplitPane` (published) | Resizable regions recur in complex TUIs | Horizontal/vertical split; min sizes; remembered fraction; divider focus/drag; collapse; tiny-area safety | Catalog/workbench and editor/results |
| `VirtualGrid` | Large two-dimensional data is broadly reusable | Borrowed visible cells; stable row/column IDs; header/gutter; two-axis viewport; range selection; column widths; hit regions; caller render projection; no fetching/edit policy | PostgreSQL table/result grid |
| `TextArea` | Multiline editing is a general primitive | Grapheme-safe buffer; cursor/selection; undo/redo; line numbers; search; vertical/horizontal scroll; paste; external spans/diagnostics; no parser | SQL and Redis command editor |
| `CompletionMenu` | Anchored candidate lists recur with editors/forms | Stable candidates; selected ID; clamp/flip geometry; scroll; keyboard/mouse; caller ranking and commit | SQL/Redis completion |
| `Progress` | Long operations recur outside databases | Determinate/indeterminate state; label/count; non-color status; cancel affordance composition | Connect, query, import/export |
| Grid/tree scrollbars | Scroll ownership is generic | Visible range/unknown total; horizontal and vertical state; drag/page behavior; painted-geometry hit regions | Grid, tree, inspectors |

### Deliberately TableRock-local

| Local component/model | Why it does not belong in TermRock |
|---|---|
| `CatalogModel` | Database object hierarchy, permissions, lazy requests, revisions |
| `DataGridModel` | Typed values, result pages, editability, mutation markers, sort/filter plans |
| `QueryEditorModel` | SQL/Redis dialect, statement selection, parameters, completion sources |
| `ValueInspectorModel` | Database types, byte/JSON projections, edit rules |
| `ChangeReviewModel` | Engine-specific transactional/mutation/TTL outcomes |
| `ConnectionFormModel` | Engine capabilities, TLS, credential sources, safety policy |
| `ServerOverviewModel` | PostgreSQL/ClickHouse/Redis measurements and permissions |

The local model renders through TermRock `Tree`, `VirtualGrid`, `TextArea`,
`Form`, dialogs, panels, status, and actions. It does not implement competing
focus, keymap, scrolling, hit-testing, or terminal lifecycle behavior.

## Application architecture

TableRock follows the same architecture shape Jackin proves, with TableRock
types and stricter database backpressure:

```text
Crossterm input ----+
engine events ------+--> Message --> update(Model, Message)
clock/signals -------+                    |
                                          +--> Model mutation
                                          `--> Vec<Effect>
                                                   |
                                              effect executor
                                                   |
                                             Rust engine/service

Model -----------------------------------------> pure View
                                                    |
                                              TermRock widgets
                                                    |
                                               Ratatui Frame
```

### Model

- Owned by one TUI task; no shared mutable presentation state.
- Contains visible state, stable focus IDs, modal stack, viewport positions,
  immutable engine snapshots/pages, revisions, and in-flight operation IDs.
- Contains no database client, socket, task handle, secret, or borrowed driver
  row.

### Message

- Semantic intent/results, not raw key meanings spread through screens.
- Terminal events are translated by the currently focused component/keymap.
- Engine events carry session/query/context/revision IDs so stale delivery is
  rejected deterministically.
- Resize, paste, mouse, tick, shutdown, effect completion, and stream closure
  are explicit messages.

### Update

- Synchronous and deterministic; no `.await`, file, process, socket, secret
  resolution, database call, or telemetry export.
- Mutates only the model and returns TermRock `UpdateResult<Effect>`.
- Applies modal precedence and focus routing in one place.
- Emits typed effects for every external action.

### Effects and subscriptions

- The Tokio executor owns database/service calls, persistence, 1Password,
  files, clipboard adapter requests, and telemetry.
- Every effect has an operation ID, cancellation path, timeout/budget, and
  redaction class.
- Terminal input, engine events, signals, and optional animation ticks are
  separate subscriptions merged by the run loop.
- Channels are bounded. Progress may be coalesced; terminal state, completion,
  failure, and safety decisions may not be silently dropped.
- Dropping a UI future is not reported as confirmed server cancellation.

### View

- Borrows immutable model state and renders one complete frame.
- Performs no I/O, state transition, task spawn, clock read, or logging.
- Renders only resident pages/rows and precomputed syntax/value projections.
- Derives mouse hit regions from painted geometry. Frame-scoped geometry is
  refreshed as one pure preparation step before input uses it.
- A redraw occurs on dirty state, resize, meaningful progress, or an active
  animation deadline; there is no unconditional high-frequency render loop.

### Terminal owner

`tablerock-cli` owns exactly one terminal session. Because TableRock needs
mouse capture and bracketed paste, it uses TermRock's Crossterm session adapter
with a Ratatui terminal rather than stacking a second initializer over it. One
Crossterm `EventStream` supplies keyboard, mouse, resize, focus, and paste; the
adapter converts each event into backend-neutral input/root messages. Setup is
fallible and restoration is attempted on normal exit, error, signal, and panic.
Partial initialization and double-restore are tested.

## Input and focus rules

- Stable semantic IDs, never durable row indices.
- `Tab`/`BackTab` move between regions; arrows move inside the focused region.
- Editors and inputs consume printable keys before global shortcuts.
- Tabs are one focus stop; arrows select within the strip, then focus moves to
  tab content.
- Hover never steals keyboard focus.
- Scroll applies only when the pointer/focus is inside the owning region.
- Disabled actions are neither keyboard nor mouse targets.
- Every shortcut has a visible reachable action; aliases stay out of the
  primary hint bar.
- Color is never the only state cue.

## TermRock contribution gate

Every new or changed TermRock primitive must have:

1. product-neutral names, borrowed render data, stable interaction IDs, and no
   TableRock/Jackin/database vocabulary;
2. behavior tests for keyboard, mouse, focus, disabled state, empty state,
   clipping, minimum rectangles, and Unicode display columns;
3. a canonical lookbook story and deterministic preview;
4. documentation that states caller-owned policy and lifecycle;
5. no Tokio/database/process dependency in TermRock;
6. a performance budget for grids/editors or other hot paths;
7. Jackin compatibility verification when an existing API changes;
8. a buildable DCO-signed commit pushed directly to TermRock `main`, with no
   branch or pull request;
9. an exact-revision TableRock pin plus committed lockfile in a later buildable
   TableRock `main` checkpoint.

## Delivery sequence

| TermRock checkpoint | Contents | Unblocks |
|---|---|---|
| T0 | Pin current revision; prove minimal TableRock shell and render harness | Architecture baseline |
| T1 | `Form`, `Tree`, `SplitPane`, scroll/hit-region extensions | Profiles and catalog |
| T2 | `VirtualGrid` with benchmark and lookbook corpus | Browsing/results |
| T3 | `TextArea` and `CompletionMenu` with Unicode/editor corpus | SQL/Redis workbench |
| T4 | `Progress` and any measured accessibility/interaction fixes | Long operations and hardening |

Do not implement the whole wishlist speculatively. Each checkpoint starts from
the next approved TableRock interaction contract, but its resulting primitive
must remain independently reusable.

## Rejected directions

- importing Jackin product crates or copying its application state;
- forking TermRock inside TableRock;
- wrapping every TableRock screen as one giant TermRock component;
- putting SQL, database types, result fetching, mutation policy, or secrets in
  TermRock;
- direct terminal escape emission from widgets when a typed TermRock request
  exists;
- a per-cell widget object tree or per-cell async fetch;
- branch dependencies or floating `main` Git dependencies;
- visual imitation of TablePro screenshots.
