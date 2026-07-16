# TermRock migration 0023: list multi-select ownership

Date: 2026-07-17

TableRock advances its exact TermRock `main` pin from `d7c998a` to `e46458a`
and audits sequential migration `0023-v0.11.0-list-multiselect-contract.md`.

## Before

`ListState` exposed mutable semantic and render-owned fields. Checkbox gestures
returned payload-free `Outcome::Changed`, forcing consumers to rescan selection.

## After

`ListState` fields are private and accessed through semantic cursor, hover,
focus, scroll, regions, and ordered-selection methods. Shared
`Outcome::CheckToggled(Id)` carries the stable toggled identity for both keyboard
and painted-checkbox gestures. No compatibility fields or payload-free check
result remain.

TableRock currently composes no TermRock `ListState`; its shell hit regions are
product-specific render output, not TermRock list internals. Therefore the
migration requires no source edit. Future profile/object lists must construct
`ListState::new`, use semantic methods, and handle identity-bearing outcomes.

Evidence: exact remote diff, repository-wide consumer search, refreshed pin and
lockfile, and full TableRock gates. No external-product source or protected
expression influenced this migration.
