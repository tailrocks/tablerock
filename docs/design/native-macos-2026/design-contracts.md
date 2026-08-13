# Native Workbench design contracts

## Ownership

- `TableRockFeature` owns stable presentation contracts and application ports.
- `TableRockPresentation` owns SwiftUI views, presentation state, and narrow
  AppKit adapters. It depends only on `TableRockFeature`.
- `TableRockBridge` translates between Swift contracts and the synchronous
  UniFFI facade. It contains no presentation code.
- Rust owns database clients, safety, redaction, paging, executable mutation,
  persistence, and terminal truth.

Development-only routes are compile-time excluded from Release builds. They
may project deterministic production models for tests but cannot supply live
behavior.

## Shell and surface hierarchy

- The toolbar carries connection context and global actions.
- The leading catalog preserves engine and object orientation.
- Tabs own object and query workspaces.
- Data grids, SQL editors, results, and inspectors are opaque content planes.
- The bottom rail exposes mode, row/page state, and pending changes.
- Sheets own connection setup, row editing, and reviewed apply.

Minimum, typical, and expanded windows preserve a usable catalog and primary
content plane. Inspectors may collapse before primary work becomes unusable.

## Engine truth

- PostgreSQL exposes relational data, structure, typed values, and reviewed
  primary-key row updates.
- ClickHouse exposes analytical catalog, data, structure, query, and mutation
  truth without implying PostgreSQL transaction behavior.
- Redis exposes keys, types, TTL, bounded scan position, collections, and
  command results; it never renders a relational grid as engine truth.

Loading, empty, disconnected, connection-error, query-error, read-only,
selected-value, pending-change, safe-review, destructive-review, and expiry
states require explicit presentations.

## Interaction and safety

- Keyboard, menu, toolbar, and direct-manipulation commands converge on the
  same application actions.
- Native tables own selection, scrolling, column resizing/reordering,
  accessibility, and context menus; they contain no backend behavior.
- Changes stay local until review. Apply uses Rust-issued bounded authority and
  becomes unavailable after consumption or expiry.
- Destructive actions require exact-name or typed confirmation according to the
  Rust safety contract.
- Credentials, SQL text, and cell values are not logged or persisted by
  presentation code.

## Material and accessibility

- Use system Liquid Glass only for functional chrome.
- Never apply glass, decorative blur, or shadow to content planes.
- Use semantic colors, native fonts, labeled icon controls, visible focus, and
  complete keyboard traversal.
- Increase Contrast, Reduce Transparency, Reduce Motion, light/dark appearance,
  inactive windows, and minimum-size layouts must preserve hierarchy and
  meaning without color-only signals.

## Clean-room provenance

TablePro public material informed broad workbench rhythm only. Apple platform
components determine native behavior. TableRock names, models, safety rules,
geometry, copy, and implementation are original. No external source, tests,
comments, branding, assets, bundle internals, credentials, or proprietary data
are used.
