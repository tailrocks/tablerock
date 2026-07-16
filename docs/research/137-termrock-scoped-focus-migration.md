# TermRock Migration 0026 Adoption

Date: 2026-07-17

## Published change

TermRock migration `0026-v0.11.0-scoped-focus-ring.md` removes `FocusState`,
`FocusOwner`, and `ButtonFocus`. `FocusRing<Id, ScopeId>` now owns typed focus
scope, per-frame target registration, traversal, reconciliation, pointer
transfer, and modal focus restoration. Consumers must register the layout truth
painted by the current frame; fixed consumer-owned focus cycles are obsolete.

The exact TableRock pin is TermRock `main` revision
`17723dbe56ddb64c4333e8ac5a7377a38c1616b0`. That revision also graduates the
public columnar `Table` widget. The table API is additive and has no migration
file. TableRock will consume it when Phase 4 introduces bounded database result
pages; the empty Phase 1 shell has no fabricated result-table state.

## TableRock migration

Before:

- `Model` stored a single `FocusRegion` value.
- TableRock owned hard-coded `next` and `previous` cycles.
- pointer messages assigned the local value directly.
- rendering geometry and the focus cycle could evolve independently.

After:

- the root TEA `Model` owns one `FocusRing<FocusRegion, FocusScope>`;
- Tab and BackTab traverse through TermRock's neutral key contract;
- every completed render sends typed `FrameRendered(ShellGeometry)` through the
  sole root update path;
- the reducer begins a focus frame, registers the active screen's ordered
  targets with canonical painted areas and enabled state, then reconciles;
- the connection picker omits its absent catalog region, and the explicit
  too-small layout disables every target; and
- pointer messages retain stable semantic targets and request them through the
  ring instead of mutating consumer-owned focus state.

The first frame is bootstrapped with the connection order and no geometry so
keyboard input remains deterministic before painted regions arrive. All later
registration derives from immutable render geometry. I/O remains outside
`update` and `view`; no component owns application state.

TableRock has no modal presentation yet. Future dialogs must use
`FocusRing::open_modal`, `pop_modal`, or `clear_modals`; presentation code must
not create a second modal/focus lifecycle.

## Verification

- TUI reducer, rendering, geometry, and focus-order suites pass.
- CLI mapping, runtime, root-ingress, fault-restoration, and real PTY lifecycle
  suites pass against the refreshed pin.
- Clippy passes with warnings denied for both presentation crates.
- The four real PTY cases prove semantic quit, signal restoration,
  resize-authorized focus, and high-rate event fairness.

## Provenance

External concept: scoped per-frame focus registration and columnar widget
graduation  
Public source: TermRock migration 0026, public API, tests, and documentation at
revision `17723dbe56ddb64c4333e8ac5a7377a38c1616b0`  
TableRock requirements: research 11, 13, 30, 31, and 32  
Implementation source: TableRock root TEA model, render geometry, reducer, and
independent PTY tests  
Copied code/assets/text: none
