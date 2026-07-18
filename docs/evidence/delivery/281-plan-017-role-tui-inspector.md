# Plan 017 residual — role effective-membership TUI projection

Date: 2026-07-18

## What landed

- `DriverSession::role_inspector_lines(schema, table)` — default
  `EngineMismatch`; PostgreSQL implementation on `PostgresSession`
  - `current_user` member
  - bounded role list + membership edges
  - effective roles via `RoleMembershipGraph` + self-cycle flag
  - optional table privilege projection when base relation known
- `SessionSlot` forwards the method
- TUI: `ActionId::ShowRoles` → `Effect::LoadRoles` → inspector
  `RolesSnapshot` (title `roles`)
- CLI executor: PG-only gate, maps engine lines to `EngineMsg`
- Unit: `show_roles_emits_load_roles_with_base_table`
- Side fix: workspace `tokio` enables `process` so `pg_process`
  (evidence 275) compiles without relying on dev-dep feature
  unification

## Commands

```bash
cargo test -p tablerock-tui show_roles
cargo check -p tablerock-cli
cargo test -p tablerock-engine --lib
```

## Residual

- DDL structure-panel quick actions
- Startup Write/Dangerous review UI
- Full pg_dump/pg_restore real-server matrix when CI has client binaries
