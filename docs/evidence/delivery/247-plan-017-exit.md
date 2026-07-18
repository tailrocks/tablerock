# Plan 017 exit — scoped parity expansion (Phase 10)

Date: 2026-07-18

| Sub-package | Status | Evidence |
|-------------|--------|----------|
| DDL typed plans + PG execute | landed | core ddl + postgres_real ddl_add_column |
| Roles list | landed | list_roles Docker |
| Relationship graph | landed | core RelationshipGraph tests |
| Vim mode layer | landed | vim_mode unit suite |
| Tool discovery pg_dump argv | landed | tool_discovery tests |
| SSH russh adapter + bastion proof | landed | 260-plan-017-russh-ssh-tunnel (known-hosts/profile residual) |

## Verification

```text
cargo test -p tablerock-core --lib ddl
cargo test -p tablerock-tui --lib vim_mode
cargo test -p tablerock-cli --lib tool_discovery
cargo test -p tablerock-engine --test postgres_real ddl_add_column
```
