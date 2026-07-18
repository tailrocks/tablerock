# Connect session registration and stub workbench

Date: 2026-07-18

## Checkpoint

Plan 006 (partial). Editor **Connect** opens a temporary live session:
connect → describe → register in process-local `SessionRegistry` →
`Screen::Workbench` with session facts. **Disconnect** removes and shuts
down the session; no durable profile is written for temporary connect.

## Decision

- `Effect::ConnectSession { temporary }` and `Effect::DisconnectSession`.
- CLI `EffectExecutor` owns `Arc<Mutex<SessionRegistry>>` (capacity 64).
- Shared `open_described_session` for Test and Connect; Test shuts down,
  Connect registers.
- Presentation-local `SessionFacts` (session id hex, identity, temporary,
  engine label, status) — no engine types in the TUI model.
- Editor Connect is always temporary in this checkpoint; list-row Open that
  loads a saved profile is deferred.

## Bounds and failure truth

- Connect failures project redacted labels; no registry entry on failure.
- Disconnect of unknown/busy session → `DisconnectFailed` label.
- Temporary connect never calls `create_profile`.

## Evidence

- `cargo test -p tablerock-tui` including
  `update::tests::connect_opens_workbench_and_disconnect_returns`.
- `cargo test -p tablerock-cli`.

## Remaining work

- List Open from saved profile + secret resolve + non-temporary path.
- Password prompt modal; PG/CH password secret bags.
- Groups Tree, TermRock Form, reconnect, removal safety, Docker Test matrix.
