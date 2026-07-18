# Phase 3 connection experience exit

Date: 2026-07-18

## Checkpoint

Plan 006 / ROADMAP Phase 3 exit. Connection list, editor, Test, Connect,
temporary session, password prompt, remove confirm, Form/Tree, and
describe_server real matrix are on `main`.

## Decision

Phase 3 is closed for the basic loop. Deferred (explicit, product-staged):

- URL import / temporary URL drafts
- 1Password / Keychain / env secret sources
- Group rename dialog (delete group + assign on edit exist)
- Wall-clock delayed reconnect re-dispatch (policy + stop-on-auth proven)

## Evidence chain

| # | Topic |
|---|---|
| 199 | Effect executor bridge |
| 200 | Test Connection effect |
| 201 | Connect + stub workbench |
| 202 | List selection / Open profile |
| 203 | TermRock VirtualGrid pin (parallel T2) |
| 204 | Form/Tree screens |
| 205 | Password prompt + reconnect policy |
| 206 | describe_server Docker matrix |

## Verification

- `cargo test -p tablerock-tui -p tablerock-cli`
- `cargo test -p tablerock-engine --test describe_server_real`
