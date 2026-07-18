# Plan 018 residual — external URL open

Date: 2026-07-18

## Threat model (deep link / hostile input)

| Attack class | Mitigation |
|--------------|------------|
| Non-DB schemes (`javascript`, `file`, `data`, …) | `HostileInput` reject |
| C0 controls / NUL in URL | `HostileInput` before parse |
| Shell metacharacters in host (`;\|&\`…`) | `InvalidHost` / `HostileInput` |
| Oversized URL | `TooLarge` at 4 KiB |
| Credential leakage in UI | `safety_summary` never includes password text; raw URL only re-parsed on OPEN |

## What landed

### Core
- Hostile scheme list + control-byte reject
- Host validation (length, path chars, shell meta)
- `ConnectionUrlDraft::safety_summary()` redacted operator line
- Units: hostile schemes/controls; summary redaction

### TUI
- `ActionId::OpenExternalUrl`
- Two-phase dialog: paste URL → review summary → paste `OPEN`/`YES`
- On confirm: temporary `ConnectSession` (never saves profile) **or**
  `ConnectProfile` when engine+host:port/db matches a loaded list row
  (evidence 297)
- Units: happy path temporary connect; hostile scheme; matched profile

## Commands

```bash
cargo test -p tablerock-core connection_url
cargo test -p tablerock-tui open_external
```
