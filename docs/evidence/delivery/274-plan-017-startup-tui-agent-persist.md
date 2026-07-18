# Plan 017 residual — startup TUI + connect report; SSH agent preference persist

Date: 2026-07-18

## What landed

### SSH agent preference
- `ProfilePreferences::ssh_use_agent` + `with_ssh_use_agent`
- Migration `0012-ssh-use-agent-preference.sql` (schema v12)
- Persist create/update/load; `aggregate_to_draft` / `draft_to_aggregate`

### Startup TUI + report
- Editor field **Startup SQL (ReadOnly lines)** — newline-separated statements
- `#` comments ignored; validates into `StartupActionSet`
- Draft carries set into connect/test
- `TestOk` / `ConnectOk` include optional `startup_summary`
  (`startup Nok/Nskip/Nfail/Ntimeout`)
- Test status and session status surface the summary

## Commands

```bash
cargo test -p tablerock-core --lib
cargo test -p tablerock-persistence --tests
cargo test -p tablerock-tui --lib
cargo test -p tablerock-cli --lib
```

## Residual

- Multi-line safety class picker (Write/Dangerous still review-only at executor)
- Full pg_dump matrix / DDL review UI (other 017 residuals)
