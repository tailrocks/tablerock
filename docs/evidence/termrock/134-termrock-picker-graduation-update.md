# TermRock Picker graduation update

Date: 2026-07-17

## Decision

TableRock advances its exact TermRock `main` pin from
`0d85cfd17c6f00d7dc279cb6ad92f39e8d6c4f70` to
`56b856be6195cdd632c546334016df02fb9aaeff` after reviewing the complete
published delta. No sequential migration follows 0025 because these changes
are additive and the later commit changes only lookbook-internal routing.

TermRock graduates the prior lookbook-local picker experiment into the public
`Picker`, `PickerState`, and `PickerOutcome` API. It composes the canonical
`TextInput` and stable-ID `List`, owns query/list input precedence and painted
pointer geometry, preserves stable selection through projection changes, and
leaves matching, ranking, candidate lifecycle, overlays, and async work to the
consumer. Direct tests cover empty, narrow Unicode, interaction, allocation,
and a 10,000-row hot path. Generated component documentation and previews are
updated in the same upstream commit. The following lookbook refactor routes
Picker pointer events through the shared internal `PointerTarget` seam without
changing the public API.

TableRock does not create a temporary Phase 2 consumer. The searchable
connection experience is a Phase 3 deliverable and depends on Phase 2 exit.
When Phase 3 begins, connection/profile selection must use this public Picker;
TableRock will own profile matching and secret-free projection only. It must not
retain a product-local generic picker composition.

## Verification and provenance

- complete upstream commit and public API reviewed
- no new migration or removed API
- TableRock full workspace, lint, rustdoc, dependency, English, and drift gates
- no external product internals or protected expression imported

Public sources:

- <https://github.com/tailrocks/termrock/commit/4a073ce9d046cb15cc3840fcfe1c91ee1f75a437>
- <https://github.com/tailrocks/termrock/commit/56b856be6195cdd632c546334016df02fb9aaeff>
- <https://github.com/tailrocks/termrock/blob/4a073ce9d046cb15cc3840fcfe1c91ee1f75a437/crates/termrock/src/widgets/picker.rs>
