# Plan 017 residual — encrypted SSH private key + TUI key field

Date: 2026-07-18

## What landed

- `SshPublicKeyAuth::from_openssh_private_key_with_passphrase` via
  `russh::keys::decode_secret_key` (wrong/missing passphrase → `Auth`)
- Connect path: when private key set, `ssh_password` is key passphrase
- TUI `SshPrivateKey` field; validate password **or** private key
- Unit: encrypted key without/wrong/correct passphrase
- Docker: encrypted key auth to bastion

## Commands

```bash
cargo test -p tablerock-engine ssh_tunnel
cargo test -p tablerock-engine --test ssh_tunnel_real encrypted_private
cargo test -p tablerock-tui --lib model::editor
```

## Residual

- SSH agent auth
