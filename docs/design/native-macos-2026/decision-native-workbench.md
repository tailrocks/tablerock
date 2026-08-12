# Native concept selection

Status: refinement complete; operator confirmation pending; production not authorized

Decision date: 2026-08-12  
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

## Authority boundary

This decision authorizes only presentation refinement inside
`TableRockDesignLab`, using static invented data and local state. It does not
authorize production UI edits, migration, backend wiring, persistence, network
access, database access, or reuse of Design Lab fixtures in production.

After refinement, the second gate must present runtime captures, interaction
evidence, component/material ownership, accessibility results, differences,
and unresolved limits. Production work may begin only after the operator then
states explicitly:

> This native concept is the production design TableRock should implement.

That second gate is now open with the exact build and evidence in the
[Native Workbench refined confirmation gate](../../evidence/design-lab/native-workbench-refined/README.md).

## Clean-room provenance

TablePro public pages and documentation informed broad user-visible workbench
organization only. Apple platform behavior and the installed stable SDK govern
native implementation. No TablePro source, tests, comments, bundle internals,
branding, assets, product copy, or proprietary fixtures were used.
