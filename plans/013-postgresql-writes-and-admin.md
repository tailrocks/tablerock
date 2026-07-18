# Plan 013: PostgreSQL writes — staged edits, review, transactional apply, admin (Phase 6)

> **Executor instructions**: Work-package plan. Read
> `docs/product/editing.md` (authority) and delivery-plan.md "Phase 6" before
> starting. Checkpoints below are trunk commits with evidence. STOP
> conditions binding. Update `plans/README.md` when done.
>
> **Drift check (run first)**: plans 011 + 012 DONE per `plans/README.md`;
> `git diff --stat d8b113b..HEAD -- crates/tablerock-core/src/mutation.rs`
> — the mutation typestate must still match "Current state" below.

## Status

- **Priority**: P1
- **Effort**: L
- **Risk**: HIGH (writes; safety-critical)
- **Depends on**: plans/011, plans/012
- **Category**: direction (Phase 6 roadmap)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

First write capability. The core typestate pipeline already exists and is
tested — `MutationPlan → review(token) → ReviewedMutationPlan →
authorize(consume-once) → AuthorizedMutationPlan` plus
`MutationReviewRegistry` (`crates/tablerock-core/src/mutation.rs:287-611`) —
but nothing executes a plan: there is no `CommandIntent` mutation variant
(core survey flagged this explicitly) and no PostgreSQL apply path. Redis has
the only executor precedent: `apply_reviewed_ttl_mutation`
(`crates/tablerock-engine/src/redis.rs:1606`) — study it as the pattern for
"authorized plan in, typed outcome out".

## Spec anchors (inline)

- Editability only from one base table with stable row identity; joins/
  aggregates/key-less/unknown/truncated read-only WITH stated reason
  (`editing.md` "Editability").
- Staging in memory per tab; survives scroll/page/sort/filter; markers
  inserted/modified/deleted with original value reachable; status-bar count;
  undo + discard (`editing.md` "Staging").
- Review dialog lists exact parameterized statements grouped by table;
  execution uses the typed plan, never reparsed preview text; apply-all or
  discard-all; per-change discard from staged view.
- Apply (PostgreSQL): one tab's set in a single transaction; zero/multiple
  affected rows = conflict → rollback, report, keep staged state; generated
  values reconcile after apply; grid refreshes.
- Ambiguous outcome stays unknown, never retried, recorded (evidence
  163–166 semantics are locked).
- Unsaved-change policy: one modal authority for close/switch/disconnect/
  quit.
- Safety modes: ReadOnly profiles show NO edit affordances; ConfirmWrites
  stages freely + requires review; destructive table ops are separate gates.
- Phase 6 delivery adds: FK navigation, table operations
  (refresh/rename/truncate/drop gates), activity/dashboard with
  permission-aware cancel/terminate, structure/index/constraint facts +
  first reviewed DDL.

## Current state (entry gate)

- Core: full mutation typestate + registry (above);
  `MutationChange::{InsertRow, UpdateRow, DeleteRow, …}`,
  `MutationExecutionModel` distinguishes PG-atomic (`mutation.rs:13`).
- Engine: `CommandSafety::MayWrite` exists (plan 002); no mutation intent,
  no PG write executor.
- Grid: identity facts + typed cells (plans 009/012); `ValueKind`-typed
  editors do NOT exist yet.
- `docs/architecture/shared-client-contract.md` "Cross-adapter conformance":
  the review-token registry will be exposed over UniFFI later — keep the
  seam move-only/handle-based (already is).

## Scope (checkpoints)

1. **Core intent + apply seam**: `CommandIntent::ApplyMutations` carrying a
   review-token handle (NOT plan bytes), Context scope, safety
   `MayWrite`-or-stronger; engine `DriverSession` write method for
   PostgreSQL taking `AuthorizedMutationPlan` (mirror the Redis TTL
   executor's shape) → typed `MutationApplyOutcome` (per-change
   applied/conflict facts, generated values, transaction terminal state,
   `Unknown` on ambiguity).
2. **PG executor**: single transaction; parameterized statements from the
   typed plan (reuse plan 012's quoting/parameter discipline); affected-rows
   verification (≠1 → rollback + conflict report); RETURNING for generated
   value reconciliation; ambiguity mapping locked to evidence-163/164
   semantics (timeout after dispatch → `Unknown`, no replay). Real-server
   suite: happy path, conflict (concurrent update), constraint violation,
   ambiguity injection (deferred-trigger fixture from
   `tests/postgres_real.rs` ambiguous-commit test), session usability after
   each.
3. **Editability proof**: extend catalog/browse plan with pk/unique facts
   (started in plan 012); `EditabilityFacts` on each result; reasons
   rendered.
4. **Staged-edit model + typed cell editors**: `MutationDraftModel` per tab
   (insert/update/delete drafts, undo stack, markers, counts); inline
   editors per `ValueKind` (bool/number/text/temporal/enum/JSON/bytes —
   unknown/invalid/truncated non-editable); ReadOnly profiles render no
   affordances (absence test).
5. **Review + apply UI**: review dialog (TermRock `Dialog` + `DiffView`
   projection) listing exact parameterized statements with values
   (bounded/redaction-aware preview from the typed plan — never
   reconstruct-by-parse); apply → registry authorize (consume-once) →
   effect → outcome rendering incl. conflict-keeps-staged and
   unknown-outcome record; unsaved-change modal integration.
6. **Admin surfaces**: FK navigation (follow FK from cell → filtered browse
   tab using plan 012's builder); table ops behind typed destructive gates
   (truncate/drop/rename with specific confirmation); activity view
   (`pg_stat_activity` snapshot, permission-aware
   `pg_cancel_backend`/`pg_terminate_backend`); structure tab
   (columns/indexes/constraints facts + raw DDL).

**Out of scope**: ClickHouse/Redis writes (plans 014/015), structure DDL
*editing* (Phase 10), backup/restore (Phase 10), import (plan 016).

## Commands

Standard suites + Docker PG. Phase 6 exit exercises delivery-plan gates:
hostile identifiers/values cannot alter operation structure (extend the
plan-012 adversarial suite to mutation plans); multi-change apply
all-or-rollback; joins/aggregates/no-key stay read-only; refresh/quit cannot
silently discard; ambiguous writes never retry.

## Done criteria

- [x] `ApplyMutations` intent + PG executor; plan bytes never cross the seam (handle-based) — evidence 227
- [x] Real-server: atomic apply, conflict rollback keeps staged state — evidence 227; generated-value reconciliation + ambiguity inject still open
- [x] ReadOnly profile: staging blocked / drafts discarded (unit tests) — evidence 228
- [x] Review dialog parameterized preview from typed plan (not reparsed text) — evidence 229
- [x] Destructive ops gated by specific confirmation (reducer test: no bypass path) — evidence 232
- [x] Adversarial quote_ident unit test for mutation SQL — evidence 230-era + d270439
- [ ] Generated-value RETURNING reconciliation + ambiguity → `Unknown` inject suite
- [ ] Consume-once registry survives UI dialog clock (re-review on expiry)
- [ ] Full ValueKind editors + cancel/terminate activity gates + rename
- [x] Suites green for landed checkpoints; ledger + ROADMAP Phase 6 partial; plan IN PROGRESS

## STOP conditions

- The consume-once registry semantics don't survive the UI round trip
  (token expiry during dialog) without weakening — STOP; expiry policy is a
  core decision (`ReviewError::Expired` exists — surface re-review, never
  bypass).
- Generated-value reconciliation requires re-keying rows the identity facts
  can't support — STOP.
- Any path would execute preview text — STOP (hard product rule).

## Maintenance notes

- Plans 014/015 implement their executors against the same intent/seam with
  their own execution models (progressive/async/sequential — already
  distinguished by `MutationExecutionModel`).
- Reviewer focus: transaction boundaries, affected-row verification,
  ambiguity mapping, token consume-once, absence-not-disabled for ReadOnly.
