# Tested support matrix

Status: 2026-07-21. This matrix records exercised configurations, not a wider
compatibility promise. A configuration absent from the **Runtime proof** column
is unproven.

## Database servers

| Engine | Tested lines | Runtime proof | Known boundary |
|---|---|---|---|
| PostgreSQL | 17.10, 18.4 | Real-server adapter, service, cancellation, TLS, projection, and native vertical tests | Test images use exact minor tags but are not digest-pinned; other server lines are unproven |
| ClickHouse | 25.8.28.1 LTS, 26.3.17.4 LTS | Digest-pinned real-server adapter, service, compression, projection, and overlap tests | Other server lines and non-LTS releases are unproven |
| Redis | 7.4.9, 8.8.0 | Digest-pinned real-server RESP2/RESP3, TLS, ACL, scan, Pub/Sub, reconnect, and overlap tests | Other server lines and compatible third-party servers are unproven |

CI exercises each real-server suite independently and a concurrent
three-engine overlap suite. The ignored live UniFFI suites are also run by CI;
an ignored test is not silently treated as proof.

## Applications and platforms

| Surface | Tested configuration | Proof | Unproven / unsupported |
|---|---|---|---|
| TUI runtime | Linux x86_64 self-hosted runner; `TERM=xterm-256color` PTY contract | Full CI, real PTY lifecycle, input, rendering, and three-engine integration | Other terminal emulators, `tmux`, SSH presentation, Linux arm64 runtime, and macOS terminal runtime |
| CLI release packages | Linux x86_64/arm64 and macOS x86_64/arm64 targets | Cross-build and package validation | Cross-built targets do not establish runtime compatibility |
| Native macOS runtime | macOS 26.4 arm64, Xcode 26.6, Swift 6.3.3 | Xcode unit/UI suites, accessibility audit, archive, Instruments time/RSS trace, and leak gate | Earlier macOS releases and Intel runtime |
| Native universal archive | arm64 and x86_64 slices | Both slices compile and the universal archive is inspected | x86_64 execution remains unproven |
| 1Password CLI | 2.35.0 | Scheduled native gate installs the current stable cask and verifies the `op read --no-newline` plus global `--account` command surface used by TableRock | Authenticated account access is operator-owned and is not exercised in CI |

The native app currently has developer/CI delivery status. Ad-hoc archive and
local install mechanics are proven. Developer ID signing, notarization,
stapling, Gatekeeper validation on a clean machine, and production update
delivery remain blocked until signing authority exists. The native app is not a
Mac App Store product.

## Local persistence

- Current schema: 18.
- Proven: fresh initialization, ordered migration ledger, interrupted-migration
  rollback, crash recovery, group backfill, and rejection of future or gapped
  ledgers.
- Not yet proven: upgrade from an actually shipped earlier TableRock release.
- Downgrade is unsupported; newer schema state fails closed.

## Maintenance proof

Scheduled workflows exercise the full CI matrix, registry and Git dependency
freshness, action pins, native Xcode/Swift/macOS suite, current 1Password CLI
surface, server matrix, accessibility checks, performance/RSS/leak gates, and
artifact metadata. A passing scheduled run proves only the versions and hosts
recorded in its artifact metadata.

Detailed commands, host metadata, failures, and remaining work live in the
[evidence index](evidence/README.md).
