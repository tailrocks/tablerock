# TermRock Form/Tree on connection screens

Date: 2026-07-18

## Checkpoint

Plan 006. Connection list renders with TermRock `Tree` (group branches +
profile leaves, collapse state). Connection editor renders with TermRock
`Form` (General / Connection / Credentials / TLS sections). No local
generic Form/Tree widgets.

## Decision

- `ConnectionFormModel` remains TableRock-local domain state; Form is the
  renderer only (`termrock-integration.md` deliberately-local table).
- Tree node ids: `g:{group}` branches, `p:{profile_id_hex}` leaves.
- Collapsed groups stored in `ProfileListState::Loaded.collapsed`.
- Selection is profile-id sticky across filter changes.

## Bounds and failure truth

- Empty list still explicit via status line; Tree paints nothing.
- Form fields are exactly the Phase 3 first-version set.

## Evidence

- Architecture test `connection_screens_use_termrock_form_and_tree`.
- `cargo test -p tablerock-tui -p tablerock-cli`.

## Remaining work

- Password prompt modal + resume effect.
- Reconnect backoff stop-on-auth.
- Docker Test matrix; Phase 3 ledger/ROADMAP close.
