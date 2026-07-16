# TermRock textarea spike update

Date: 2026-07-17

TableRock advances its exact TermRock `main` pin from `87b6a28` to `a9774f5`.
The published commit adds only a lookbook textarea spike, its design plan, and
the lookbook-local `unicode-segmentation` dependency. The public `termrock`
crate and published sequential migrations are unchanged, so TableRock requires
no source migration.

The spike proves grapheme-safe cursor movement, selection, insertion/deletion,
line indexing, preferred visual columns, viewport tracking, and bounded undo
history as forward design evidence. TableRock does not import lookbook internals;
any reusable editor primitive must later land as a documented neutral TermRock
API with its own sequential migration.

Evidence: exact published diff `87b6a28..a9774f5`, unchanged public library
surface, refreshed lockfile, and full TableRock workspace gates. Uncommitted
TermRock work from the concurrent library agent was not read as published API,
modified, staged, or overwritten. No external-product source or protected
expression influenced this update.
