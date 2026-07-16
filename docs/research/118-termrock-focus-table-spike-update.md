# TermRock focus and table spike update

Date: 2026-07-17

TableRock advances its exact TermRock `main` pin from `51910bf` to `87b6a28`.
The two published commits add lookbook-only scoped-focus and deterministic-table
spikes plus design plans. They do not change the public `termrock` crate, its
features, or its sequential migration guide, so TableRock requires no source
migration.

The focus spike explores hierarchical scope ownership, restoration, modal
trapping, and deterministic directional movement. The table spike explores
typed columns, stable sorting, horizontal/vertical windows, row identity,
selection, resize, and narrow-width projection. These are usage evidence only;
TableRock continues using published library APIs and does not import lookbook
product internals.

Evidence: the exact remote diff `51910bf..87b6a28`, unchanged public crate
surface, refreshed lockfile, and the complete TableRock workspace gates. No
external product source or protected expression influenced this update.
