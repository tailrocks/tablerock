# TermRock 0.10 Content Revision Migration

Status: accepted and integrated on 2026-07-16.

TableRock advances its exact TermRock `main` pin from `5c4758b` to
`4b7927335663e0b275588a93ca0ebe6bc4032b0d` (`0.10.0`) and applies sequential
migration `0013-v0.10.0-content-measurement-revisions.md` after `0012`.

## Old to new

Large borrowed `Viewport` and `DetailTable` widgets can opt into cached content
measurement by supplying a stable domain-owned `content_revision`; consumers
must bump it whenever content changes. Length changes invalidate automatically,
while omitting the revision retains always-measure behavior.

`DialogScroll` and `DetailTableState` now contain private derived measurement
caches. Consumers migrate state literals to `Default`, then restore only public
domain interaction fields such as scroll offsets and selection. Derived
measurement fields are never persisted.

TableRock does not yet instantiate these primitives, so no consumer source
rewrite is required in this checkpoint. Future catalog/grid composition will
use root-model content revisions, default-constructed widget state, and
non-persisted TermRock caches. No compatibility wrapper or parallel cache exists.

## Verification

- Source search finds no existing `Viewport`, `DetailTable`, or `DialogScroll`
  construction requiring migration.
- Root TEA, render, interaction, bounded-ingress, and PTY suites pass at the
  exact revision.
- Workspace tests, Clippy with warnings denied, and rustdoc pass.

External concepts: revision-keyed derived measurement caches
Public source: <https://github.com/tailrocks/termrock/blob/4b7927335663e0b275588a93ca0ebe6bc4032b0d/migrations/0013-v0.10.0-content-measurement-revisions.md>
Implementation source: TermRock migration and TableRock consumer audit
Copied code/assets/text: none
