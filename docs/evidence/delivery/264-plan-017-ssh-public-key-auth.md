# Plan 017 residual — SSH public-key auth

Date: 2026-07-18

## What landed

- `SshAuthMaterial::{Password, PublicKey}` on `SshTunnelConfig`
- `SshPublicKeyAuth::from_openssh_private_key` — unencrypted OpenSSH PEM;
  Debug redacts key material
- `authenticate_publickey` via russh `PrivateKeyWithHashAlg`
- Real Docker: bastion with `PasswordAuthentication no` + authorized_keys;
  public-key auth succeeds; password auth returns `SshTunnelError::Auth`

## Commands

```bash
cargo test -p tablerock-engine ssh_tunnel
cargo test -p tablerock-engine --test ssh_tunnel_real public_key
```

## Residual

- Agent auth
- Encrypted private keys + passphrase prompt
- Profile aggregate SSH properties + connect-path auto-wrap
