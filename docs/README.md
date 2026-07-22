# TableRock Documentation

TableRock is a terminal-first, multi-model database workbench for PostgreSQL,
ClickHouse, and Redis. These documents are the single source of truth for what
is being built, why, and what has been proven so far.

Three layers, read in this order:

| Layer | Question | Where |
|---|---|---|
| Product | What does the operator see and do? | [`product/`](product/README.md) |
| Architecture | How is it built and why? | `architecture/` |
| Evidence | What is proven so far? | [`evidence/`](evidence/README.md) |

[`../ROADMAP.md`](../ROADMAP.md) sequences the work.
[`prompt.md`](prompt.md) is the canonical `/goal` prompt that authorizes and
drives the whole program.
[`support-matrix.md`](support-matrix.md) records only configurations exercised
by repository evidence and names the unproven boundaries.

## Product specification

Screen-by-screen behavior, written before and independent of implementation:

- [Product overview](product/README.md)
- [Connections](product/connections.md) — list, groups, environment tags,
  editor, test/connect.
- [Workbench](product/workbench.md) — context bar, sidebar catalog, tabs.
- [Data grid](product/data-grid.md) — sorting, filtering, columns, paging.
- [Editing and review](product/editing.md) — staged changes, SQL preview,
  apply.
- [SQL editor](product/sql-editor.md) — query tabs, autocomplete, results.
- [Copy and export](product/copy-export.md)
- [Redis screens](product/redis.md) · [ClickHouse screens](product/clickhouse.md)
- [Native macOS experience](product/native-macos.md)

## Architecture decisions

Stable, curated design documents. Each records one selected approach;
alternatives appear only where a rejection needs justification.

Product and process:

- [Vision and scope](architecture/vision-and-scope.md) — product boundary and
  non-goals.
- [Product identity](architecture/product-identity.md) — naming and positioning.
- [Clean-room reference policy](architecture/clean-room-reference.md) — how
  TablePro, TablePlus, and Zedis may (and may not) inform this product.
- [Workflow inventory](architecture/workflow-inventory.md) — the user workflows
  the workbench must serve; superseded screen-by-screen by `product/`.
- [Database capability model](architecture/database-capabilities.md) — what each
  engine can and cannot do.
- [Redis reference analysis (Zedis)](architecture/redis-reference-zedis.md)
- [Functional parity ledger](architecture/functional-parity-ledger.md) — the
  feature baseline every release is measured against.
- [Canonical screen manifest](architecture/screen-manifest.tsv) and
  [state profiles](architecture/screen-state-profiles.tsv) — machine-readable
  interface traceability and honest open replay gaps.

System design:

- [Application pattern: TEA](architecture/application-pattern.md) — The Elm
  Architecture as the sole TUI pattern.
- [Rust core architecture](architecture/rust-core-architecture.md)
- [Shared Rust/client contract](architecture/shared-client-contract.md)
- [Terminal experience](architecture/terminal-experience.md)
- [Native macOS path](architecture/native-macos-path.md) — SwiftUI/AppKit over
  synchronous UniFFI.
- [Primary-source platform ruling](architecture/platform-architecture-sources.md)
- [TermRock integration and extensions](architecture/termrock-integration.md)

Dependencies and delivery:

- [Fixed decisions](architecture/fixed-decisions.md) — the approved,
  change-only-by-revision decision list.
- [Delivery plan](architecture/delivery-plan.md) — detailed checkpoint
  deliverables and phase gates.
- [Dependency evaluation](architecture/dependency-evaluation.md) ·
  [Dependency policy](architecture/dependency-policy.md)
- [Quality and verification](architecture/quality-and-verification.md)
- [Main-branch delivery](architecture/main-branch-delivery.md) — trunk-only,
  forward-only workflow.

## Evidence

Every completed checkpoint records an evidence document — decision, bounds,
failure truth, verification, remaining work. The [evidence
index](evidence/README.md) groups them by phase and topic in chronological
order.

## Rules for maintaining these docs

- One decision, one place. Product spec owns *what*, architecture owns *how*,
  evidence owns *proven*. Link instead of duplicating; the roadmap links to
  evidence, evidence links to architecture, never the reverse.
- Update the relevant product, architecture, or evidence document in the same
  commit as the behavior change.
- New completed checkpoint → new numbered evidence document plus one line in
  the evidence index. Do not append changelogs to the roadmap or this map.
