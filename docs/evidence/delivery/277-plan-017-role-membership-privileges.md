# Plan 017 residual — role membership and table privileges

Date: 2026-07-18

## What landed

- `PostgresSession::list_role_memberships` — direct `pg_auth_members` edges
  (role → member), bounded limit
- `PostgresSession::list_table_privileges` — `information_schema.table_privileges`
  → `RolePrivilegeRow` (grantee, privilege, object, grantable)
- Docker: create roles + GRANT membership + GRANT SELECT; assert membership
  edge and privilege row

## Commands

```bash
cargo test -p tablerock-engine --test postgres_real role_memberships
```

## Residual

- Recursive inheritance expansion + self-lockout tests before mutations
- TUI projection of membership/privilege lists
