# Phase 2 Execute intent and StatementText

Date: 2026-07-18

## Checkpoint

Plan 002 step 1. Core gains an operator-supplied statement path for the
shared command envelope without a statement-classification parser.

## Decision

- `StatementText`: owned UTF-8, reject above `MAX_STATEMENT_BYTES` (1 MiB),
  custom `Debug` reports only `bytes` (never SQL text).
- `CommandIntent::Execute { statement }` is valid only under
  `CommandScope::Context(_)`.
- New `CommandSafety::MayWrite`; `Execute` maps to it. Unknown statements are
  treated as writes until a later parse-backed classification lands.
- `CommandIntent` / `CommandEnvelope` drop `Copy` so the envelope can own
  statement text. Accessors take `&self`.

## Bounds and failure truth

- Oversized statement fails closed before I/O with `StatementTextError::TooLarge`.
- Scope mismatch returns `CommandBuildError::ScopeMismatch`.
- Redaction class remains `MetadataOnly`; Debug of intent/envelope never embeds
  statement body.

## Evidence

- `cargo test -p tablerock-core` — includes statement bounds, Debug redaction,
  Execute scope/safety, and existing envelope tests (intent clone in scope
  matrix).
- `cargo check --workspace --all-targets` after the non-Copy envelope change.

## Remaining work

- Session registry and runtime borrow (plan 002 step 2).
- PostgreSQL/ClickHouse statement streaming + health (step 3).
- Multi-operation real-server proof (step 4).
