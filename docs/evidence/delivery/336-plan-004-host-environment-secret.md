# Plan 004 / 006 residual — host environment secret source

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `resolve_for_connect` HostEnvironment reads `std::env` | done |
| Missing/empty env → `EnvVarMissing` (fail closed) | done |
| Resolved value zeroized; Debug redacts payload | done |
| Editor `HostEnvironment` source + env var name validation | done |
| Draft/profile round-trip keeps env *name*, not value | done |
| Connect resolves env at attempt time (Redis credentials) | done |
| 1Password / Keychain remain unsupported (fail closed) | done |

## Decision

Host environment is safe to ship without OS keychain integration: the
profile stores only the variable name; resolution happens at connect/test
and the value is never persisted. 1Password CLI and Keychain stay
`SourceNotYetSupported` until operator/tooling gates are met.

## Evidence

```text
cargo test -p tablerock-engine --lib host_environment
cargo test -p tablerock-engine --lib keychain_still
cargo test -p tablerock-tui --lib host_environment_password
cargo check -p tablerock-cli
```

## Remaining work

- 1Password CLI (`op read`) and macOS Keychain sources
