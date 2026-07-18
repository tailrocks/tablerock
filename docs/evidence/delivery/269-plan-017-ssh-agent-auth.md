# Plan 017 residual — SSH agent authentication

Date: 2026-07-18

## What landed

- `SshAgentAuth` + `SshAuthMaterial::Agent`
  - `from_env` → `SSH_AUTH_SOCK`
  - `from_socket_path` → explicit Unix socket (tests/fixtures)
- `authenticate_with_agent`: list identities, try each via
  `authenticate_publickey_with` (signing stays in agent)
- Real Docker proof: in-process russh agent server seeded with test key;
  bastion pubkey-only; `connect_session` succeeds

## Commands

```bash
cargo test -p tablerock-engine ssh_tunnel
cargo test -p tablerock-engine --test ssh_tunnel_real agent_auth
```

## Follow-up landed (same day)

- TUI `SshUseAgent` toggle + draft `ssh_use_agent`
- Connect path uses `SshAgentAuth::from_env` when toggle set

## Residual

- Persist agent preference as a profile property (session intent only today)
