# Bounded real-server test concurrency

Date: 2026-07-21

## Failure

CI run 29849426803 passed dependency audit and all container-free checks. The
real-server lane then started 43 tests across four binaries at nextest's
CPU-derived default concurrency. Its first ClickHouse test exhausted the
35-second readiness deadline before the server accepted queries; the remaining
42 tests were cancelled by fail-fast.

## Correction attempt

The combined real-server matrix now caps nextest at four concurrent tests with
`-j 4`. This preserves cross-engine overlap coverage while preventing an
unbounded startup burst from competing for Docker CPU, memory, storage, and
network readiness. Per-test readiness deadlines remain unchanged and honest.

Run 29851685110 disproved concurrency as the complete cause: its Velnor lane
failed the same first ClickHouse readiness probe with `-j 4`. The cap remains a
resource-safety bound, but evidence 594 records the actual reachability defect
and correction.

## Verification

- Installed nextest 0.9.140 help confirms `-j, --test-threads <N>` controls the
  number of simultaneously running tests.
- Run 29849426803 independently passed container-free format, lint, check, and
  tests plus dependency audit.
- Bounded Velnor run 29851685110 reproduced the failure.

## Provenance

No external product reference influenced this CI scheduling correction.
Evidence comes from TableRock's hosted nextest log and installed CLI contract.
