# TermRock semantic inventory update

Date: 2026-07-17

TableRock advanced its exact TermRock `main` pin from
`e46458ac9e8145dbc5fb89f9f27d29ced8816b0c` to
`1ebac0147d160bff02cec805954537fb02d52d31` after reviewing the complete
published delta.

The TermRock change replaces generated placeholder rustdoc with semantic API
descriptions, corrects the documented component inventory to include
`LogPane` and `Progress`, and extends the catalog checker to reject placeholder
documentation. Public signatures and runtime behavior do not change. No new
sequential migration exists or is required after migration 0023, and TableRock
needs no source adaptation.

Uncommitted sibling-worktree changes, including a draft migration 0024, were
not consumed or modified. TableRock adopts only immutable published TermRock
`main` revisions.

Verification rebuilds TableRock against the exact new revision and runs the
workspace, TUI, CLI, lint, and rustdoc gates. This review uses only the
published TermRock commit and TableRock-owned tests; Jackin product internals
remain excluded.
