# Phase 2 Profile Engine and Favorite Filter Evidence

## Checkpoint

The one bounded profile-list contract now accepts a closed
`ProfileListFilter`: optional engine plus optional favorite state. All four
filter shapes—neither, engine, favorite, and both—share the same immutable
summary page and stable ordering.

Continuation cursors now carry their exact filter scope. Core rejects a cursor
under a different scope as `CursorFilterMismatch` before persistence I/O. This
is an intentional forward API replacement: callers pass
`ProfileListFilter::default()` for the former unfiltered request and retain the
same filter for every continuation.

## Migration and query contract

Sequential persistence migration `0005-profile-engine-list-index.sql` adds the
engine-prefixed canonical keyset index. Its separate before/after document is
linked after `0004` in `MIGRATING.md`.

The adapter constructs query shape only from closed enum/boolean filter facts.
All facts, cursor components, IDs, and limits remain bound parameters. No user
text is interpolated. Engine requests seek through the `0005` index;
unfiltered/favorite requests retain the `0004` index. Every query still fetches
at most `limit + 1` rows and never selects secret payload columns.

## Evidence

- Core tests bind a continuation to Redis/favorite and reject reuse under the
  unfiltered scope.
- PostgreSQL, ClickHouse, and Redis engine filters each return only their stable
  profile ID.
- Favorite-only pagination with limit one returns two profiles across a scoped
  continuation; non-favorite returns the remaining profile.
- ClickHouse plus non-favorite returns an honest empty final page.
- `EXPLAIN QUERY PLAN` proves unfiltered and engine-filtered shapes use their
  respective documented indexes.
- Existing least-data and debug-redaction assertions remain unchanged.

## Deliberate boundary

Engine and favorite filters are complete below presentation. Exact group/tag
filters, normalized search, endpoint display facts, health state, and UI loading/
empty/failure projections remain required. Future filters must join this scoped
cursor contract rather than create a parallel list API.

## Verification record

- `cargo test -p tablerock-core --test profile_list`: 3 passed.
- `cargo test -p tablerock-persistence --test profile_create`: 7 passed.
- `cargo test --workspace --all-targets --locked`: 102 passed, 3 ignored.
- Workspace format, Clippy with warnings denied, rustdoc, both query-plan index
  checks, diff, English-only, redaction, and provenance review: pass.

External concepts: filter-scoped keyset pagination and closed query-shape construction
Public sources: no new external source; contract derives from approved TableRock architecture and ledger
Implementation source: TableRock-owned core contract, migration, adapter, and tests
Copied code/assets/text: none
