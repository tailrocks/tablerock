# Connection editor Test Connection effect

Date: 2026-07-18

## Checkpoint

Plan 006 (partial). Editor can run Test without saving: connect →
`DriverSession::describe` → shutdown; result projects into editor
`test_status` only. No session registration, no profile create.

## Decision

- `Effect::TestConnection { request_token, draft }` carries the presentation
  draft; secrets live only inside the CLI executor task for the duration of
  the attempt.
- Completions: `EngineMsg::TestOk { identity, elapsed_millis }` /
  `TestFailed { reason: FailureProjection::Label }` — redacted labels only.
- Action bar on Editor: Save / Test / Cancel / Quit.
- Password field added to editor cycle; displayed as `••••` when non-empty.
- Redis Test passes credentials when username/password present. PostgreSQL
  connect config still has no secret bag (trust fixtures work); ClickHouse
  password bag deferred to same engine gap.
- TLS draft modes map Off → disabled; VerifyCa/VerifyFull → required (system
  roots). Custom-CA UI remains out of scope.

## Bounds and failure truth

- Invalid port fails before network I/O with label `invalid port`.
- Adapter/connect failures project as redacted labels (no credential bytes).
- Test never calls persistence create/replace.
- Session is shut down after describe (PG inherent; CH/Redis via boxed trait).

## Evidence

- `cargo test -p tablerock-tui` including
  `update::tests::test_action_emits_effect_and_records_outcome`.
- `cargo test -p tablerock-cli` (PTY suite green).

## Remaining work

- Connect / temporary session / disconnect (registry + stub workbench).
- Password prompt modal for `PromptOnConnect` (secret never in Model).
- Engine secret bags for PG/CH password on Test.
- Groups UI, TermRock Form/Tree, reconnect backoff, removal safety.
- Docker real-server Test matrix for all three engines.
