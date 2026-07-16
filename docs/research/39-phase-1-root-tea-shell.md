# Phase 1 Root TEA Shell Evidence

## Checkpoint

TableRock now has one root `Model`, `Message`, `update`, `Effect`,
`Subscription`, and `View` path. The reducer is synchronous and deterministic;
it owns resize, focus traversal, action selection/activation, redraw, and exit
intent without performing I/O. The model contains presentation facts only.

The shell projects wide, medium, narrow, and explicit minimum-size states. Focus
order is root-owned and wraps across context, catalog, tabs, content, actions,
and footer. The view borrows the model and composes TermRock `Panel`, `Tabs`,
`ActionBar`, `HintBar`, and `StatusBar` primitives. Focus has a text/glyph cue
independent of color, and the visible hint vocabulary follows the focused
region. No component owns application state and no generic widget layer exists
inside TableRock.

Subscriptions declare one terminal input, one signal source, and a bounded
256-event engine source. This checkpoint defines their stable presentation seam;
the CLI executor and event merge remain the next checkpoint.

## Evidence

- Reducer tests cover changed/unchanged resize, forward/backward focus,
  root-owned action selection/gating, activation, and exit effect emission.
- Breakpoint tests cover wide, medium, narrow, width-minimum, and height-minimum
  behavior.
- `TestBackend` full-frame tests cover wide, medium, narrow, and minimum-size
  composition plus narrow focus projection and non-color cues.
- Workspace tests, formatting, strict Clippy, rustdoc, diff hygiene, and a CJK
  natural-language scan pass.

This checkpoint does not claim terminal lifecycle, live input, panic/signal
restoration, dirty-frame scheduling, or PTY completion.

External concept: The Elm Architecture and responsive terminal composition only  
Public sources: <https://ratatui.rs/concepts/application-patterns/the-elm-architecture/> and <https://github.com/tailrocks/termrock/tree/8cb3c88d118b2cbed10eef9d7cdbf0c0adbbbfde>  
TableRock requirement: Roadmap Phase 1 / delivery-plan root TEA shell  
Implementation source: TableRock research decisions, TermRock public API, and independent tests  
Copied code/assets/text: none
