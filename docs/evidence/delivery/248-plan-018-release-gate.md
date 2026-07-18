# Plan 018 exit — TUI hardening / Phase 11 gate

Date: 2026-07-18

## Gate map (quality-and-verification.md → proof)

| Requirement | Proof |
|-------------|--------|
| Redaction / no secrets in Debug | `tablerock-core` test `redaction_audit` |
| Non-color state cues | `tablerock-tui` test `non_color_cues` (all GridOperationState labels) |
| OTLP off by default | `tablerock-cli` telemetry module + unit test |
| Release profile build | `cargo build -p tablerock-cli --release` (this checkpoint) |
| Suite classes exist | unit/model/engine Docker/PTY already in tree (001–017) |

## Residual (explicit)

- Full failure-injection matrix as scheduled CI
  - partial: export write fail-closed (evidence 280)
  - partial: SIGWINCH/resize storm unit (evidence 285)
  - partial: readonly-parent / mid-write fail-closed (evidence 287)
- Performance budgets
  - partial: local-rig release cold-start + binary size (evidence 288)
  - partial: unit first-paint &lt; 50 ms (evidence 297)
  - partial: Docker first-page/stream budgets PG/CH/Redis (evidence 301)
  - partial: ubuntu-latest CI runs performance_real + pg_dump_real (evidence 303)
  - residual: multi-OS fixed-spec matrix publish
- Ledger three-state export: landed CSV + counts (evidence 286)

## Parity claim status

**TUI program: features exist with residual polish; not a marketing parity
claim.** Core largely implemented; Parity gaps listed in evidence 286 block a
full marketing parity claim. Native blocked on plan 019 packaging.
