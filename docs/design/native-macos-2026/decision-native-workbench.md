# Native concept selection

Status: confirmed for production implementation

Decision date: 2026-08-12  
Confirmation date: 2026-08-12

Decision owner: operator

## Decision

The operator selected **Native Workbench** at the first native-design gate after
launching the current Design Lab preview. Native Workbench is now the only
concept to refine through Phases 10 and 11.

The selection approves its structural direction:

- persistent leading database catalog;
- compact connection/database context;
- document-style object and query tabs;
- opaque, dense data/editor workspace;
- native result table and optional trailing inspector;
- continuously visible safe-mode and pending-change context.

Query Studio, Column Observatory, Grid Canvas, and Change Desk remain comparison
evidence. Their parts are not implicitly approved for remixing.

## Reasoning record

The operator named Native Workbench after previewing the runnable concepts. No
additional selection rationale or requested remix was supplied. Earlier
operator direction established TablePro's coherent native macOS workbench as
the preferred visual and interaction reference; Native Workbench is the
clean-room concept that most directly fits that direction.

## Refined-concept confirmation

After reviewing the runnable refined Design Lab concept and its Phase 11
evidence, the operator stated exactly:

> This native concept is the production design TableRock should implement.

The confirmation applies to the exact Native Workbench implementation captured
from commit `73dfd349084a9d13a0f0437656542a318f69e4df` and presented by the
Phase 11 gate at commit `53f33c42474355a09805c2b43df3a0a9426f5ce5`.

No remix, exception, or requested visual change accompanied the confirmation.

## Authority boundary

This decision now authorizes production architecture planning, refactoring, and
visual migration to the confirmed Native Workbench. Production must preserve
Rust ownership, synchronous UniFFI, safety/redaction below presentation, and
the existing functional contracts.

The Design Lab remains reference evidence only. Production must not depend on
its fixtures, state, or source. Approved parts must be deliberately implemented
in production-owned files behind the final ownership boundaries. The confirmed
build and evidence remain available in the
[Native Workbench refined confirmation gate](../../evidence/design-lab/native-workbench-refined/README.md).

## Clean-room provenance

TablePro public pages and documentation informed broad user-visible workbench
organization only. Apple platform behavior and the installed stable SDK govern
native implementation. No TablePro source, tests, comments, bundle internals,
branding, assets, product copy, or proprietary fixtures were used.
