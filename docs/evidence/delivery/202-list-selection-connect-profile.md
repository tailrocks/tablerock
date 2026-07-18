# Connection list selection, search, and Open profile

Date: 2026-07-18

## Checkpoint

Plan 006 (partial). Connections list supports selection, client-side search,
and **Open** of a saved profile into a registered session.

## Decision

- `ProfileListState::Loaded` gains `selected` + `search`.
- Content focus + Activate cycles selection; paste on Content appends search.
- Action **Open** on Connections emits `Effect::ConnectProfile` for the
  selected row (empty list → no-op).
- CLI loads `ProfileAggregate`, resolves plaintext password via
  `resolve_for_connect` (prompt source fails closed with
  `password prompt required` before network I/O), maps to `ConnectionDraft`,
  then reuses Connect path with `temporary: false`.

## Bounds and failure truth

- Prompt-on-connect fails before connect with a source label only.
- Missing profile / bad id → ConnectFailed label.
- Search is local over loaded rows (name, host:port, group, engine).

## Evidence

- `cargo test -p tablerock-tui -p tablerock-cli`.

## Remaining work

- Password prompt modal; TermRock Form/Tree groups; reconnect; removal;
  Docker Test matrix; Phase 3 ledger/ROADMAP close.
