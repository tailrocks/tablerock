# TableRock Documentation

TableRock is a terminal-first, multi-model database workbench for PostgreSQL,
ClickHouse, and Redis. These documents are the single source of truth for what
is being built, why, and what has been proven so far.

## Start here

- [`prompt.md`](prompt.md) — the canonical `/goal` prompt that authorizes and
  drives the whole program.
- [Vision and scope](architecture/vision-and-scope.md) — product boundary and
  non-goals.
- [Product identity](architecture/product-identity.md) — naming and positioning.
- [`../ROADMAP.md`](../ROADMAP.md) — phase plan and current status.

## Architecture decisions

Stable, curated design documents. Each records one selected approach;
alternatives appear only where a rejection needs justification.

Product and process:

- [Clean-room reference policy](architecture/clean-room-reference.md) — how
  TablePro, TablePlus, and Zedis may (and may not) inform this product.
- [Workflow inventory](architecture/workflow-inventory.md) — the user workflows
  the workbench must serve.
- [Database capability model](architecture/database-capabilities.md) — what each
  engine can and cannot do.
- [Redis reference analysis (Zedis)](architecture/redis-reference-zedis.md)
- [Functional parity ledger](architecture/functional-parity-ledger.md) — the
  feature baseline every release is measured against.

System design:

- [Application pattern: TEA](architecture/application-pattern.md) — The Elm
  Architecture as the sole TUI pattern.
- [Rust core architecture](architecture/rust-core-architecture.md)
- [Terminal experience](architecture/terminal-experience.md)
- [Native macOS path](architecture/native-macos-path.md) — SwiftUI/AppKit over
  synchronous UniFFI.
- [Primary-source platform ruling](architecture/platform-architecture-sources.md)
- [TermRock integration and extensions](architecture/termrock-integration.md)
- [Shared Rust/client contract](architecture/shared-client-contract.md)

Dependencies and delivery:

- [Dependency evaluation](architecture/dependency-evaluation.md) — per-dependency
  decisions.
- [Dependency policy](architecture/dependency-policy.md) — latest-stable rule
  and freshness gate.
- [Delivery plan](architecture/delivery-plan.md) — detailed checkpoint
  deliverables and phase gates.
- [Fixed decisions](architecture/fixed-decisions.md) — the approved,
  change-only-by-revision decision list.
- [Quality and verification](architecture/quality-and-verification.md)
- [Main-branch delivery](architecture/main-branch-delivery.md) — trunk-only,
  forward-only workflow.

## Evidence

Every completed checkpoint records an evidence document — decision, bounds,
failure truth, verification, remaining work. The [evidence
index](evidence/README.md) groups them by phase and topic in chronological
order.

## Rules for maintaining these docs

- One decision, one place. Link instead of duplicating; the roadmap links to
  evidence, evidence links to architecture, never the reverse.
- Update the relevant architecture or evidence document in the same commit as
  the behavior change.
- New completed checkpoint → new numbered evidence document plus one line in
  the evidence index. Do not append changelogs to the roadmap or this map.
