# Shared operator store: CLI/TUI and native production path parity

Date: 2026-08-09

## Checkpoint

CLI open-default and native production `AppPaths` both open the same durable
operator store root and `profiles.db` filename so saved profiles, session
intent, history, and related operator data round-trip when switching clients.

## Decision

- Shared path authority lives in `tablerock_persistence::operator_paths`
  (`default_operator_profiles_database`, `resolve_operator_profiles_database`).
- macOS production: `~/Library/Application Support/TableRock/profiles.db`
  (matches native `AppConfiguration` / `AppPaths.profilesDatabase`).
- Isolation: absolute `TABLEROCK_TEST_ROOT` → `{root}/profiles.db` (CLI
  open-default; native continues to require `TABLEROCK_TEST_MODE` + absolute
  root).
- CLI no longer uses process-local `~/.tablerock/state-{pid}.db`.
- Live engine sessions remain process-local (no daemon). Switch mid-work is
  durable intent + reconnect, not shared live TCP.
- Single-writer switch discipline: quit or shut down one client before the
  other opens the same file. `PersistenceActor::shutdown`/`Drop` join the
  worker so `PathLease` releases; UniFFI bridge shutdown takes and shuts down
  the actor before clearing inner state.

## Bounds and failure truth

- Relative `TABLEROCK_TEST_ROOT` → `OperatorPathError::AbsoluteTestRootRequired`.
- Concurrent dual-writer of one Turso file is not supported; sequential open
  after clean close is the supported switch model.
- Developer ID / notarize proof remains externally blocked (plan 019/021).

## Evidence

- `cargo test -p tablerock-persistence --test shared_operator_store`
- `cargo test -p tablerock-cli --test shared_operator_store`
- `cargo test -p tablerock-ffi --test shared_store_roundtrip`
- Unit tests in `operator_paths.rs` (production layout, test root, relative reject)
- Native production path unchanged; comment documents Rust contract match
  (`AppConfiguration.swift` / `AppConfigurationTests.testProductionRoot`)

## Remaining work

- Plan 021 hosted live/IME/accessibility matrix and signed clean-machine
  release remain open or externally blocked.
- Cross-process file lock (beyond process-local `PathLease`) remains out of
  scope; single-writer switch is the product rule.
