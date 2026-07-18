# Plan 004 residual — 1Password CLI (`op read`) secret resolution

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `OnePasswordReference::secret_reference_uri` + compact wire | done |
| Engine `OnePasswordReadPort` + `OpCliReader` (`op read --account --no-newline`) | done |
| Timeout (30s), 256 KiB cap, empty/fail/missing CLI fail closed | done |
| Resolved value zeroized; Debug redacts payload; stderr drained not retained | done |
| Editor `OnePassword` source + compact wire validation | done |
| Draft/profile round-trip keeps IDs only, not resolved secret | done |
| Connect resolves via `op read` at attempt time | done |
| Keychain remains `SourceNotYetSupported` | done |

## Decision

Rust owns account-pinned `op read` for ID-based secret references:

- URI: `op://{vault}/{item}/{field}` or with section
- Account: `--account {account_id}`
- Flags: `--no-newline`
- Bounded wait and stdout cap; never log secret or stderr content

Profiles store only reference IDs + breadcrumb. Resolution happens only during
Test/Connect. Keychain stays native-only / deferred. Metadata-only 1Password
item picker UI remains residual (operator pastes compact IDs or uses saved
profile).

## Evidence

```text
cargo test -p tablerock-core --test secret one_password
cargo test -p tablerock-engine --lib secret_resolution
cargo test -p tablerock-tui --lib one_password
cargo check -p tablerock-cli
```

## Remaining work

- macOS Keychain source (Swift adapter / plan 020+)
- 1Password metadata picker (account/vault/item browse)
- Live signed-in `op read` integration matrix when operator session available
