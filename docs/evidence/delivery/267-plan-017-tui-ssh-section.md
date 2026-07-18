# Plan 017 residual — TUI connection editor SSH section

Date: 2026-07-18

## What landed

- `ConnectionFormModel` SSH fields: bastion host/port/user/password + known_hosts path
- Focus cycle through SSH fields after TLS
- Form section **"SSH tunnel"** in connection editor view
- Validation: when bastion set → port 1..=65535, known_hosts path, password required
- `connection_draft_from_editor` forwards SSH fields into connect/test effects

## Commands

```bash
cargo test -p tablerock-tui --lib
```

## Residual

- Private-key field in editor UI
- Agent auth / encrypted key passphrase
