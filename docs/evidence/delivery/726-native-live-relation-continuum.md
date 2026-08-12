# Native live Relation Continuum

Date: 2026-08-13

Checkpoint: P6b — remove the final invented Release data path

## Decision

Row Continuum now renders only live PostgreSQL rows returned through a
Rust-owned relation-browse operation. Swift sends the opaque session/object
identities, selected column, and exact versioned-page cell payload. Rust then:

1. loads the bounded FK graph and requires exactly one matching edge;
2. revalidates that exact edge against `pg_catalog`;
3. rejects incomplete graphs, NULL/truncated/structured/invalid/unknown cells,
   ambiguous edges, and composite foreign keys;
4. resolves and quotes the target column's exact catalog type;
5. binds the selected value separately as text and converts it to that type;
6. streams at most 500 related rows through the existing operation/page path.

This replaces the presentation-owned sample edge map and invented artist/album
rows. Presentation owns only explicit-open interaction, loading/error state,
the split plane, and projection of returned rows.

## Compatibility and recovery

- This is an additive UniFFI expansion. Existing catalog browse methods remain
  unchanged, so rollback is the previous Swift call path plus removal of the
  new method; no persisted data or schema migration exists.
- Debug/Test scripted support implements the same Feature protocol and remains
  compile-time excluded from Release.
- Xcode's first mixed-version build correctly failed when generated Swift knew
  the new symbol but the existing XCFramework header did not. Rebuilding the
  XCFramework restored header/library agreement before tests continued.
- A generic browse-filter implementation was rejected during review because
  it inferred parameter type from display text. The final contract instead
  preserves exact page bytes and uses the target type identity from Rust-owned
  catalog truth.

## Safety and failure truth

- No cell value is concatenated into SQL. The generated statement contains one
  `$1` placeholder; request Debug output redacts cell bytes.
- Schema, table, column, type-schema, and type-name identifiers are all quoted
  independently. Hostile identifier coverage proves escaping.
- Exact target-type conversion preserves numeric-looking text, UUID/domain,
  numeric, temporal, boolean, and binary values. Structured cells fail closed
  because their inspection projection is not a PostgreSQL input literal.
- Composite FK traversal fails closed until a complete row-identity contract
  exists. Multiple FK edges on one selected column also fail closed rather
  than choosing one silently.
- Selection changes immediately invalidate the open plane; an async result is
  discarded if its source tab or selected cell changed while I/O was pending.

## Evidence

- Rust browse-plan tests passed for bound-value non-inlining, exact target-type
  casts, hostile catalog identifier quoting, and unbounded export replay.
- Rust FFI unit tests passed for page-cell conversion/redaction and rejection
  of NULL, truncated, structured, invalid, and unknown cells.
- Rust conformance passed for outbound and inbound routing, type ownership,
  ambiguous-edge rejection, composite-key rejection, and placeholder-only SQL.
- The real PostgreSQL bridge suite passed with a text FK value containing
  significant surrounding whitespace and returned only the exact parent row.
- Focused Xcode suites passed: Feature 34/0, Presentation 3/0, App integration
  22/0. App coverage proves exact page bytes reach the Rust-owned operation,
  returned rows populate Continuum, and changing selection closes the plane.
- `scripts/build-native-app.sh`, `scripts/verify-native-source-ownership.sh`,
  `scripts/verify-native-query-tabs.sh`, SwiftPM Release tests, and the final
  Release archive/string scan passed.

## Clean-room provenance

TablePro's public user-visible relationship workflow and native spatial
composition informed the confirmed interaction direction. Implementation and
tests derive only from TableRock requirements, PostgreSQL catalog semantics,
the existing typed page contract, and direct tests. No TablePro source, tests,
comments, bundle internals, branding, assets, copy, or proprietary fixtures
were used.
