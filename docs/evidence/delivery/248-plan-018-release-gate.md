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
- Published performance budget numbers on fixed-spec runners
- Complete ledger three-state spreadsheet export

## Parity claim status

**TUI program: features exist with residual polish; not a marketing parity
claim.** Open residuals remain documented in plans 013–017 residual sections
and this gate residual list.
