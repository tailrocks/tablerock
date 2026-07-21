# 579 — Native profile-create XCUITest

Date: 2026-07-21

## Behavior

The canonical app UI test now clicks **New connection**, enters a name through
the real native text field, verifies Save becomes enabled, clicks Save, observes
the explicit `Connection created` outcome, and finds the new durable-ID profile
row in the sidebar.

The injected scripted backend retains immutable profile drafts and list rows
for the test process. A feature-model test independently proves create, list,
and draft round-trip with a stable ID/revision while ensuring no password value
is retained. Each XCUITest still owns a unique temporary application root.

## Verification

`swiftc -parse` and `git diff --check` pass locally. Full type checking and the
user-control test require Xcode 26.6 and remain pending on the hosted canonical
checkpoint after push.

## Provenance

Implementation source: TableRock's native profile requirements, injected
backend boundary, and stable accessibility contract.

TablePro influence: broad connection-management workflow only. No source,
tests, identifiers, assets, product text, screenshots, layout measurements,
colors, or key bindings were copied or translated.
