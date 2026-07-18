# Password prompt port and reconnect policy

Date: 2026-07-18

## Checkpoint

Plan 006. Prompt-on-connect fails before network I/O and opens a password
modal; submit resumes connect with the secret only inside the effect.
Reconnect backoff policy is pure and stops on authentication labels.

## Decision

- `EngineMsg::PasswordPromptRequired` when `resolve_for_connect` needs a
  prompt; Model holds `PasswordPrompt` with Debug-redacted buffer.
- `Effect::ResumeConnectProfile { password }` carries the secret once; Model
  clears the prompt on Submit before the effect is observed.
- `next_backoff_ms` / `stop_on_failure_label` in `tablerock-tui::reconnect`.
- `Effect::ReconnectSession` attempts connect; auth failures →
  `ReconnectStopped`; other failures → `Reconnecting` with next delay fact
  (sleep is declarative for UI re-dispatch, not forced wall-clock in the
  attempt path).

## Bounds and failure truth

- Unresolved prompt never reaches `open_described_session`.
- Password bytes never appear in `PasswordPrompt` Debug.
- Backoff budget: attempts 0..=5 (1s…30s); attempt 6+ exhausted.

## Evidence

- `update::tests::password_prompt_debug_redacts_buffer_and_submit_clears`
- `reconnect::tests::*`
- `cargo test -p tablerock-tui -p tablerock-cli`

## Remaining work

- Docker Test/Connect matrix for all three engines.
- Auto re-dispatch loop with real delayed sleep for reconnect.
- Phase 3 ledger + ROADMAP exit close.
