# TermRock 0.10 Metadata and Selection Migration

Status: accepted and integrated on 2026-07-16.

After applying migration `0010`, TableRock advances its exact TermRock `main`
pin from `37d1fda` (`0.9.0`) to
`ccf06463382b7ca2ca7734f3d58acdc43366fb54` (`0.10.0`). TermRock's sequential
`MIGRATING.md` links the separate
`0011-v0.10.0-trailing-metadata-cells.md` before/after guide.

## Old to new

List and Tree now own optional aligned trailing metadata cells and ordered
multi-selection. Tree adds an explicit checked-state outcome. Consumers add a
`trailing` field when constructing those rows, use state constructors, and
remove local metadata width/padding or buffer-patching logic.

TableRock's current root shell does not yet construct List/Tree rows or match
their outcomes, so no source call site requires migration. Future profile and
catalog surfaces will use the new canonical metadata/selection APIs directly;
no pre-0.10 row facade or local alignment implementation is introduced.

The same published main also adds reusable deterministic Progress and
tail-following LogPane components, matching previously documented TableRock
TermRock extension needs. They will be adopted when the owning execution/log
checkpoints begin, not duplicated locally.

## Verification

- Source inspection finds no pre-0.10 List/Tree construction or exhaustive
  outcome match.
- Root TEA rendering, input, and PTY fixtures pass at the exact 0.10 revision.
- Workspace tests, Clippy, and rustdoc pass.

External concepts: widget-owned trailing metadata, ordered multi-selection, and reusable progress/log views
Public source: <https://github.com/tailrocks/termrock/tree/ccf06463382b7ca2ca7734f3d58acdc43366fb54>
Implementation source: TableRock dependency pin only
Copied code/assets/text: none
