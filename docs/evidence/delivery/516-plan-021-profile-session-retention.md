# Plan 021 — native profile session state and deletion retention

Date: 2026-07-19

## Structural correction

Saved-profile open previously rebuilt generic connection parameters and then
registered a random profile identity. Session behavior worked, but Rust could
not relate that session back to its durable profile. `open_profile` now carries
the exact saved `ProfileId` through driver registration. Generic temporary
connections still receive independent transient identities.

`BridgeProfileItem.connected` is derived from Rust's live session registry.
Swift renders that fact; it does not infer domain state. Successful connection
replacement disconnects the previous presentation-owned session only after the
new session opens. Failed replacement therefore leaves current work intact.
Native row menus and the workbench toolbar expose explicit disconnect.

Profile deletion still removes only profile-owned rows. Query history has no
profile ownership and remains present. An already-open session keeps its owned
driver/session/context scope after profile deletion and can finish an operation
before explicit disconnect.

## Evidence

| Gate | Result |
|---|---|
| exact saved `ProfileId` session registration | pass |
| connected profile projection | pass |
| delete profile, execute through retained session, disconnect | pass |
| query history retained after profile deletion | pass |
| persistence focused deletion test | pass |
| UniFFI conformance | pass; 17 tests, 5 ignored |
| native group structural/runtime gate | pass; connected projection included |
| native accessibility structural/runtime gate | pass |

Periodic driver health probing, reconnect-state projection, and authentication
stop-state coverage remain later Plan 021 gates.

## Provenance

TablePro was used only to confirm the broad concepts of visible connection
state and preserving active work when saved connection metadata changes. No
source, tests, text, screenshots, layouts, measurements, colors, assets, or key
bindings were copied or translated.
