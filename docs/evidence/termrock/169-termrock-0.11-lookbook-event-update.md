# TermRock 0.11 Lookbook Event Update

## Upstream inspection

TableRock refreshed its TermRock `main` pin from `5ff94ee` to `b7f34da` before
publishing the PostgreSQL JSON checkpoint. The intervening commits complete
lookbook interactor event-name convergence, mark planning verification records
current, and reconcile resolved audit findings through plan 037.

The production `termrock` crate, public API, migration index, and migration
files are unchanged. The code delta is confined to the independent lookbook and
documentation. No TableRock source migration is required.

## Adoption

The workspace pins exact revision
`b7f34da8db5842bb439296fe4cde534de0c1eb3c`, still version 0.11.0 with the
`crossterm` and `serde` features. Lockfile refresh and an all-feature workspace
check pass. TableRock retains the canonical TermRock runner, input vocabulary,
focus, tree, table, form, overlay, and text contracts without compatibility
shims.

External concepts: none; upstream dependency drift inspection only
Public sources: <https://github.com/tailrocks/termrock/compare/5ff94ee117fd4a1b72fdd0d1b1847815055a93ac...b7f34da8db5842bb439296fe4cde534de0c1eb3c>
Implementation source: exact TermRock main history and TableRock dependency manifests
Copied code/assets/text: none
