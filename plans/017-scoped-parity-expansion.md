# Plan 017: Scoped parity expansion — SSH tunnels, pg_dump/restore, structure editing, roles, editor polish (Phase 10)

> **Executor instructions**: Work-package plan; this is a BUNDLE of six
> independent sub-packages — execute in the order below but treat each as
> its own evidence-gated checkpoint series. Authority: ROADMAP Phase 10,
> delivery-plan.md "Phase 10", fixed-decisions.md ("SSH and cloud
> transport"), parity-ledger "Later" rows. STOP conditions binding. Update
> `plans/README.md` when done.
>
> **Drift check (run first)**: plan 016 DONE.

## Status

- **Priority**: P2
- **Effort**: L (bundle)
- **Risk**: MED–HIGH (SSH + DDL are safety surfaces)
- **Depends on**: plans/016
- **Category**: direction (Phase 10 roadmap)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Sub-packages (ordered)

### A. SSH tunneling (`russh`)

Fixed decision: one Rust `russh` adapter BELOW all three database clients;
owns host-key verification/known-hosts, agent/key/password auth, local
forwarding, keepalive, cancellation, reconnect, redacted errors; drivers
receive only the established local stream/endpoint; cloud-provider
proxy/identity EXCLUDED. `russh` is a new dependency — adoption checkpoint
with version/license/MSRV/motivation, latest stable, exact pin.
Profile model: SSH settings extend the aggregate ("SSH + evidence-backed
advanced settings are later aggregate extensions" — profiles evidence 56);
editor gains an SSH section (extension, not redesign — connections.md
"Deferred" list). Ledger acceptance: "Real SSH bastion matrix; no shell
interpolation or secret logging." Tests need an SSH server fixture
(containerized sshd; follow the testcontainers patterns in
`tests/redis_real.rs`).

### B. PostgreSQL backup/restore (`pg_dump`/`pg_restore`)

Tool integration with version checks, process supervision, progress,
cancellation, file controls, secret-safe invocation (password via
env/pgpass-style mechanism, NEVER argv; no shell — spawn directly).
Builds on plan 016's `FileEffects`. Ledger acceptance: "Real
`pg_dump`/`pg_restore` matrix". External tool discovery: explicit path
setting + version probe; missing tool = explicit unsupported state.

### C. Reviewed structure/DDL editing

Capability-gated reviewed DDL: PostgreSQL first (ALTER TABLE add/drop
column, index create/drop, constraint ops), ClickHouse-specific forms where
official capabilities prove them, NO Redis fiction (explicit unsupported).
Reuses the plan-013 review seam; every operation a typed plan → review →
authorize → execute with observed outcome. Ledger acceptance:
"Destructive-operation and rollback/outcome tests" (PG DDL transactional
where PG guarantees; CH DDL non-transactional wording).

### D. Relationship exploration + roles (PostgreSQL)

FK relationship graph contract (terminal tree/list view first; the graph
CONTRACT must be native-diagram-ready per ROADMAP); role/privilege
inspection (read-only first: roles, memberships, grants per object),
reviewed changes only as a separate later checkpoint. Ledger acceptance:
cycles/large-graph/missing-FK tests; effective-privilege and self-lockout
tests before any mutation support.

### E. Startup actions + Vim mode

Reviewed bounded startup SQL/commands per profile (safety classification,
timeout, partial-failure states; explicit reconnect behavior). Optional Vim
behavior over the neutral editor state machine (TermRock TextArea must not
be forked — keymap layer in TableRock; independent keymap/mode-transition
suite per ledger).

### F. Maintenance/optimize + engine administration rows

Applicable maintenance ops behind typed gates (PG VACUUM/ANALYZE/REINDEX,
CH OPTIMIZE where permitted); every engine-inapplicable feature renders an
explicit unsupported capability (ROADMAP Phase 10 closing rule).

## Cross-cutting rules

- Every sub-package: evidence docs per checkpoint, parity-ledger row
  transitions, real-privilege/version/destructive/failure tests, explicit
  unsupported states on other engines, no generic UI inventing cross-engine
  behavior (delivery-plan Phase 10 exit).
- Dependencies (`russh`, any tool-discovery helper) each get an adoption
  checkpoint; nothing else new without STOP.

## Commands

Standard suites + Docker; new fixtures: sshd container, pg_dump/pg_restore
binaries on the CI runner (document version matrix in evidence; if CI can't
host them, keep those suites local-only and record the gap — same pattern as
`performance_real`).

## Done criteria

- [x] SSH russh adapter + password bastion Docker proof (evidence 260)
- [x] SSH known_hosts fail-closed (evidence 261)
- [x] PG/CH/Redis drivers through local-forward tunnel matrix (evidence 262–263)
- [x] SSH public-key auth (evidence 264)
- [x] Profile SSH property bindings (evidence 265)
- [x] Connect-path SSH auto-wrap + session-owned tunnel (evidence 266)
- [x] TUI connection editor SSH section (evidence 267)
- [x] Encrypted SSH private key + TUI key field (evidence 268)
- [x] SSH agent auth + TUI/connect agent toggle (evidence 269)
- [x] pg_dump discovery + argv never carries password (tool_discovery tests)
- [x] DDL plans typed (DdlPlan) + PG execute_ddl_plan add/drop column + vacuum/analyze; Redis unsupported
- [x] Roles: list_roles read-only Docker test
- [x] Startup actions core contract (evidence 270)
- [x] PG startup executor ReadOnly auto-run (evidence 271)
- [x] Startup persist + connect-path wire (evidence 272)
- [x] CH/Redis startup executors (evidence 273)
- [x] Startup TUI lines + connect report; SSH agent preference persist (evidence 274)
- [x] pg_dump/pg_restore process supervision + cancel (evidence 275; real client matrix residual)
- [x] DDL index/constraint execute (evidence 276)
- [x] DDL TUI review/authorize for add_column + create_index (evidence 278)
- [x] Role membership + table privileges (evidence 277)
- [x] Effective role inheritance expansion + self-cycle detection (evidence 279)
- [x] Role effective-membership TUI projection (evidence 281)
- [x] Startup Write/Dangerous review UI (evidence 282)
- [x] Vim mode keymap layer unit suite; off by default
- [x] Relationship graph contract + self-cycle detection
- [x] Plan index DONE with residual SSH/full dump matrix

## Residual

- Full pg_dump/pg_restore real-server matrix when CI has client binaries (process cancel landed, evidence 275)
- DDL structure-panel quick actions (full action-bar DDL review landed, evidence 278)

## STOP conditions

- russh cannot express a required auth/forwarding mode — STOP (fixed
  decision names russh; revision required).
- Any sub-package pressures a cloud-provider identity integration — STOP
  (excluded by decision).
- DDL forms require parsing user SQL back out of preview text — STOP.

## Maintenance notes

- Sub-packages are independently shippable; do not hold SSH hostage to Vim.
- Reviewer per package: A) host-key + secret hygiene; B) process/argv
  hygiene; C) review-gate completeness; D) privilege correctness; E) safety
  classification; F) unsupported-state honesty.
