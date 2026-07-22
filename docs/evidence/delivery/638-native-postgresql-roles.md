# Evidence 638: native PostgreSQL roles

Date: 2026-07-22

## Outcome

`TR-SCR-046` now exposes read-only PostgreSQL role and privilege inspection in
native macOS through a typed Rust authority:

- one bounded snapshot contains current user, roles, direct memberships,
  effective roles, cycle edges, and optional relation grants;
- roles cap at 64, memberships at 128, effective expansion at 32, and grants at
  64; any exceeded bound produces an explicit truncated state;
- optional privilege failures remain distinct from an empty grant collection;
- existing terminal presentation derives from the same typed snapshot;
- UniFFI accepts an optional opaque cached PostgreSQL relation handle and
  rejects engine/kind/stale-handle mismatches;
- native supports global inspection, selected-relation grants, search,
  loading, empty, unavailable, cycle, truncation, refresh, and close states.

Mutation stays explicitly unavailable. A separate typed plan, review,
authorization, outcome, and self-lockout gate is still required before this
screen is proven complete.

## Verification

```text
mise exec -- cargo check -p tablerock-engine --all-targets
green

mise exec -- ./scripts/build-native-app.sh --configuration Release
Built native/dist/TableRock.app
```

Focused real PostgreSQL, facade, model, XCUITest, clippy, and hosted checkpoint
results are recorded by the completion commit and its exact-sha workflows.

## Primary sources

- PostgreSQL 18 role membership catalog:
  <https://www.postgresql.org/docs/18/catalog-pg-auth-members.html>
- PostgreSQL 18 `table_privileges` view:
  <https://www.postgresql.org/docs/18/infoschema-table-privileges.html>

## Clean-room provenance

TablePro public material was checked only for the broad existence of user,
role, and privilege inspection. No source, tests, strings, assets, screenshots,
layout measurements, colors, or key bindings were copied. TableRock's typed
snapshot, bounds, states, wording, and presentation were independently designed
from repository requirements, PostgreSQL documentation, and direct tests.
