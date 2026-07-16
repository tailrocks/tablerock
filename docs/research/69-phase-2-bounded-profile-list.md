# Phase 2 Bounded Profile List Evidence

## Checkpoint

The core now owns immutable `ProfileListRequest`, `ProfileListCursor`,
`ProfileListItem`, and `ProfileListPage` contracts. Requests require 1–100
items. Pages cannot exceed the request and expose an opaque stable keyset cursor
only when a lookahead row proves more data exists.

Persistence migration `0004-profile-list-index.sql` adds the exact canonical
ordering index. Its separate sequential migration document records the old and
new access shapes in `MIGRATING.md`.

## Projection and privacy contract

Profiles order by favorite first, explicit saved order, then opaque stable ID.
Continuation seeks after that tuple; no offset scan or name-based cursor exists.
One worker-owned read transaction fetches at most `limit + 1` rows.

Each summary contains only:

- stable ID, revision, engine, validated redacted name/group;
- favorite/order and safety mode;
- literal-versus-secret facts for host and port;
- whether any secret source or dangerous plaintext source exists.

The SQL projection never selects literal text values, secret blobs,
environment names, Keychain bytes, or 1Password reference columns. It does not
resolve any source. Core constructors revalidate every returned label and
discriminator; malformed durable rows fail closed as metadata-only
`ProfileDecode` and the actor remains usable.

## Evidence

- Core tests reject zero/oversized requests, oversized pages, and empty
  continuations while proving cursor derivation and redacted debug output.
- Persistence fixtures create PostgreSQL, ClickHouse, and Redis profiles, then
  prove a two-item first page and one-item continuation across favorite groups.
- Summary source facts identify literal endpoint fields, secret-backed fields,
  and dangerous plaintext presence without exposing values.
- A database-valid whitespace-only name makes both list and single read fail
  closed; a following health command succeeds.

## Deliberate boundary

The canonical unfiltered organization list is bounded and cursor-based.
Engine/favorite filters are now implemented by
[`71-phase-2-profile-engine-favorite-filter.md`](71-phase-2-profile-engine-favorite-filter.md).
Search, tag/group filters, health facts, endpoint display projection, and UI
states remain Phase 3 work. They must extend this one contract rather than
introduce an unbounded or secret-bearing list path.

## Verification record

- `cargo test -p tablerock-core --test profile_list`: 2 passed.
- `cargo test -p tablerock-persistence --test profile_create`: 7 passed.
- `cargo test --workspace --all-targets --locked`: 101 passed, 3 ignored.
- Workspace format, Clippy with warnings denied, rustdoc, query-plan index,
  diff, English-only, redaction, and provenance review: pass.

External concepts: stable keyset pagination and least-data list projections
Public sources: no new external source; contract derives from approved TableRock architecture and ledger
Implementation source: TableRock-owned core contract, migration, adapter, actor, and tests
Copied code/assets/text: none
