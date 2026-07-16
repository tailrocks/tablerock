# Phase 2 Normalized Profile Search Evidence

## Checkpoint

The canonical bounded profile-list filter now owns an optional redacted
`ProfileSearchTerm`. Input is limited to 128 UTF-8 bytes, rejects controls and
blank normalized text, and caps normalized expansion at 1,024 bytes.

Search normalization version 1 is:

1. Unicode NFKC compatibility composition;
2. full default non-Turkic Unicode case folding;
3. Unicode NFKC compatibility composition again.

The selected `unicode-normalization` 0.1.25 tables implement Unicode 17.0; the
selected `caseless` 0.2.2 fold tables implement Unicode 16.0. Search behavior is
therefore deterministic for this exact dependency tuple and will change only
through a documented forward contract revision.

## Bound and storage decision

Raw validated profile names, groups, and tags remain authoritative. Search does
not persist derived normalized keys, so no schema migration, backfill, trigger,
or stale-key repair path is required. This checkpoint correctly adds no SQL
migration after `0006`.

At most 10,000 saved profiles may exist. Create rejects the next profile as
metadata-only `ProfileCapacity`. A search query applies indexed exact filters
and the keyset cursor first, streams at most 10,001 candidate summaries, and
fails closed if a preexisting database exceeds the cap. It retains at most
`page limit + 1` matching summaries and one profile's bounded tags while
normalizing. The existing 30-second actor deadline remains the time bound.

Search reads only name, group, and bounded tag text in addition to the existing
least-data summary. It never selects property values, secret blobs,
environment names, Keychain bytes, or 1Password reference fields. Search terms
and labels remain absent from Debug and public errors.

## Evidence

- Core tests prove compatibility matching (`ＰＲＯＤ`), canonical combining
  equivalence (`CAFÉ`), full fold expansion (`Straße`/`STRASSE`), blank/control
  rejection, cursor-scope binding, and redacted diagnostics.
- Persistence tests prove honest one-item continuations across three matches,
  group matching, tag matching, composition with engine/favorite filters, and
  an empty final result.
- A capacity boundary unit test accepts 9,999 and rejects 10,000 and larger.
- Existing exact-filter index plans, least-data source facts, malformed-row
  rejection, and three-engine tests remain green.

## Deliberate boundary

Normalized name/group/tag search is complete below presentation. Ranking is
canonical stable organization order, not fuzzy relevance. Endpoint display
facts, health state, and UI loading/empty/failure projections remain required.
If fuzzy ranking is later approved, it must remain bounded and replace this
ordering explicitly rather than run as an unbounded presentation-side scan.

## Verification record

- `cargo test -p tablerock-core --test profile_list`: 5 passed.
- `cargo test -p tablerock-persistence --locked`: 16 passed.
- `cargo test --workspace --all-targets --locked`: 104 passed, 3 ignored.
- Workspace format, Clippy with warnings denied, rustdoc, dependency-tree/license,
  architecture-boundary, diff, English-only, redaction, and provenance review:
  pass.

External concepts: Unicode compatibility normalization, full default case folding, and bounded streaming search
Public sources: Unicode Standard Annex #15; unicode-rs crate documentation and repositories
Implementation source: TableRock-owned core normalization contract and bounded persistence scan
Copied code/assets/text: none
