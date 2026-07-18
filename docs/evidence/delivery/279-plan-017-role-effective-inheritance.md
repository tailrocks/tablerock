# Plan 017 residual — effective role inheritance expansion

Date: 2026-07-18

## What landed

- `RoleMembershipEdge` / `RoleMembershipGraph` in core
  - `effective_roles(member, max)` — BFS transitive expansion, bounded
  - `has_self_cycle_through(member)` — self-lockout signal for review UI
  - unit: chain + cycle + bound
- `PostgresSession::effective_roles_for` loads membership graph from
  `pg_auth_members` then expands
- Docker: `tr_child` inherits `tr_parent` (no self-cycle)
- Circular GRANT is rejected by PostgreSQL itself; cycle logic remains pure

## Commands

```bash
cargo test -p tablerock-core role_membership
cargo test -p tablerock-engine --test postgres_real role_memberships
```

## Residual

- TUI projection of effective roles / cycle warnings
