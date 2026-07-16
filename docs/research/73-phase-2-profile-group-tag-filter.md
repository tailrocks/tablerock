# Phase 2 Profile Group and Tag Filter Evidence

## Checkpoint

The canonical bounded profile list now supports exact validated group and tag
filters alongside engine and favorite. Filter scope owns bounded core label
types; cursor/request ownership is cloneable rather than scalar-copy-only so
future normalized search can extend the same contract without a second cursor.

## Core API migration

Before, `ProfileListFilter` contained only copyable engine/favorite facts and
`ProfileListPage::new` consumed a request. Now filters may own redacted bounded
group/tag labels, page validation borrows the request, and `next()` returns a
cloned opaque cursor. Callers retain one request filter and clone it when
starting or continuing pages. No legacy cursor or parallel overload remains.

A cursor stores the entire filter scope. Changing group or tag returns
`CursorFilterMismatch` before I/O. Cursor debug output uses the labels' redacted
length-only representations.

## Migration and query contract

Sequential migration `0006-profile-group-list-index.sql` adds the exact
group-prefixed canonical ordering index. Its separate before/after document is
linked after `0005` in `MIGRATING.md`.

Exact tag filtering starts from `saved_profile_tags_lookup(tag, profile_id)`,
joins the parent by stable ID, and then applies the same bounded keyset order.
Group/tag values are bound parameters; SQL shape comes only from the closed
filter fields. The summary projection still never selects any property value or
secret-reference payload.

## Evidence

- Cursor scope includes redacted group/tag labels and rejects a changed tag.
- Exact Operations and Analytics groups return the expected stable IDs in
  favorite/order/ID order.
- Exact `cache` tag returns only the Redis profile.
- A combined Redis + favorite + Operations + cache filter returns one profile;
  incompatible combinations return an honest empty page.
- Query-plan inspection proves group and tag requests use migrations `0006` and
  the existing tag lookup index respectively.
- Existing three-engine, pagination, source-fact, and redaction proofs remain
  green.

## Deliberate boundary

Exact engine, favorite, group, and tag filters are complete below presentation.
Normalized name/group/tag search, endpoint display facts, health state, and UI
loading/empty/failure projections remain required. Search must extend this
owned scope and define Unicode normalization before schema work.

## Verification record

- `cargo test -p tablerock-core --test profile_list`: 3 passed.
- `cargo test -p tablerock-persistence --test profile_create`: 7 passed.
- `cargo test --workspace --all-targets --locked`: 102 passed, 3 ignored.
- Workspace format, Clippy with warnings denied, rustdoc, group/tag query-plan
  checks, diff, English-only, redaction, and provenance review: pass.

External concepts: owned filter-scoped cursors and indexed exact organization filters
Public sources: no new external source; contract derives from approved TableRock architecture and ledger
Implementation source: TableRock-owned core contract, migration, adapter, and tests
Copied code/assets/text: none
