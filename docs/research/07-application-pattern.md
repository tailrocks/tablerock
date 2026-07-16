# Application Pattern: The Elm Architecture

## Decision

TableRock's Rust TUI uses **The Elm Architecture (TEA)** as its sole application
pattern. It does not use Ratatui's Component Architecture or Flux.

TermRock remains the sole reusable widget/runtime library. TermRock currently
exposes a trait named `Component` as a library contract, but TableRock widgets do
not own independent application state, perform I/O, or form a Component
Architecture. All authoritative TUI state remains in one root `Model` and all
state transitions pass through the root `update` path.

## Evidence

Ratatui's official [TEA guide](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/)
defines Model, Message/Update, and View. It emphasizes predictable state
transitions and a side-effect-free view. It also permits Rust to mutate the
model in place when that is the practical performance choice; TEA does not
require cloning a database workbench model.

Ratatui's official
[Component Architecture guide](https://ratatui.rs/concepts/application-patterns/component-architecture/)
places private state, event handlers, update, and render behavior inside each
component. That is useful for independent object-like UI regions, but it is the
wrong authority model for TableRock.

Ratatui presents these pages as patterns, not a mandate. TEA is TableRock's
project decision because it matches the safety and asynchronous-state
requirements below.

## Binary comparison

| Requirement | TEA | Component Architecture | Decision |
|---|---|---|---|
| One truth for focus, modal stack, tabs, sessions, revisions, and pending writes | Root model owns it | State is distributed among components | TEA |
| Global safety invariants | One reducer path sees every intent | Requires coordination across private component state | TEA |
| Async database events | Typed messages enter one ordered update path | Event fan-out and component-local handling can diverge | TEA |
| Stale result rejection | Root update checks session/context/query/revision together | Each component must duplicate or coordinate checks | TEA |
| Cancellation truth | Operation state machine changes in one reducer | Local component lifecycle can be confused with server lifecycle | TEA |
| Modal input routing | Root update enforces one precedence rule | Multiple handlers may claim the same event | TEA |
| Deterministic tests | Message plus model yields model plus effects | Tests need component graph and cross-component action routing | TEA |
| Reusable TermRock widgets | Stateless/borrowed render data fits root state | Components can encapsulate state, but that conflicts with one authority | TEA |
| Large model performance | Rust may mutate one model in place | Local mutation is natural | Tie; not a reason to surrender global invariants |

Component Architecture is rejected because TableRock's screens are not
independent. The catalog changes completion; connection context invalidates
tabs/results; dialogs gate writes; result identity controls editing; disconnect
changes every active operation. Private state and handlers would create
coordination paths exactly where safety requires one authority.

This Ratatui pattern governs the Rust TUI. The native macOS client is a thin
projection adapter over the same Rust commands/events/pages: one `@MainActor`
presentation store receives immutable events and sends typed intent. It does not
introduce component-owned domain state or a second product behavior model.

## Required architecture

```text
Crossterm input ---+
engine event ------+--> Message --> update(&mut Model, Message)
signal/resize/tick -+                    |
                                          +--> state transition
                                          `--> Vec<Effect>
                                                   |
                                             effect executor
                                                   |
                                             Rust application service

&Model --> pure view --> TermRock widgets --> Ratatui frame
```

### Model

The root `Model` owns presentation state: active workspace, tabs, focus, modal
stack, editor/grid/catalog state, resident immutable pages, stable IDs,
revisions, operation projections, and safe notifications. It contains no driver,
socket, database row, resolved secret, Tokio handle, or Swift object.

Large feature state is organized as typed submodels for profiles, catalog,
workbench, editor, results, Redis values, and dialogs. Submodels are structure,
not independent component authorities.

### Message

One root `Message` enum contains semantic user intent, engine events, effect
completion, resize, signal, paste, mouse, and timer facts. Feature-specific
message enums may be nested for organization, but all are wrapped by and routed
through the root message path.

Raw keys do not leak through the product. The focused TermRock primitive maps
input to semantic messages under root-owned modal/focus precedence.

### Update

`update(&mut Model, Message) -> UpdateResult<Effect>` is synchronous and
deterministic. It may delegate to pure feature reducer functions, but there is
one root ordering and safety authority. It performs no file, process, secret,
database, telemetry, clock, sleep, or async work.

The reducer validates stable identity and revision before applying every async
completion. It returns typed effects and dirty state; it never calls an effect
executor directly.

### Effects and subscriptions

The Tokio-owned executor handles database/service calls, persistence,
1Password, files, clipboard adapter requests, and telemetry. Each effect carries
operation identity, budget/deadline, cancellation scope, and redaction class.

Terminal input, engine events, signals, and requested ticks are subscriptions
merged into the one root message queue. Queues are bounded. Progress may be
coalesced; state transitions and terminal outcomes may not be dropped.

### View

`view(&Model, &mut Frame)` renders the complete intended frame through TermRock
and Ratatui. It performs no I/O, state transition, task spawn, clock read, or
logging. Stateful Ratatui rendering uses frame-local projection state rather
than mutating domain/presentation truth during render.

### Source organization

```text
tui/
  model.rs
  message.rs
  update.rs
  effect.rs
  subscriptions.rs
  run.rs
  view/
    connections.rs
    workspace.rs
    catalog.rs
    editor.rs
    grid.rs
    redis.rs
    dialogs.rs
```

One file need not contain the whole reducer or view. Modules may expose pure
sub-reducers and render functions. They do not own event loops or hidden state.

## Test consequences

- reducer tests assert model transitions and emitted effects from messages;
- model invariant/property tests cover modal, stale revision, edit, and
  cancellation state machines;
- TermRock widgets use direct Ratatui `Buffer` tests;
- complete views use `TestBackend` with owned TableRock fixtures;
- effect executors use fake and real application-service ports;
- PTY tests cover the single terminal owner and lifecycle.

## Enforcement

Architecture review rejects:

- component-owned application/domain state;
- component-local database or filesystem I/O;
- multiple event loops or direct component subscriptions;
- render-time state transitions;
- raw engine events applied outside the root revision checks;
- a second generic widget/runtime layer beside TermRock;
- naming Component Architecture as a TableRock application option.
