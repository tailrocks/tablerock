# Product Specification

This directory defines **what TableRock builds**, screen by screen, in plain
terms. It is the layer between vision and architecture:

```text
product/        what the operator sees and does (this directory)
architecture/   how it is built and why (decisions, contracts, ownership)
ROADMAP.md      when each part ships (phases and gates)
evidence/       what has been proven so far
```

Read these documents first when the question is "what should the screen do".
Read `../architecture/` when the question is "how may it be implemented".

## Product in one paragraph

TableRock is a terminal-first database workbench for PostgreSQL, ClickHouse,
and Redis. Two clients share one Rust core: a CLI/TUI built on TermRock, and a
native macOS app built on SwiftUI/AppKit with Liquid Glass. The operator
creates connections, organizes them in groups, connects, browses schemas and
objects in a sidebar, views and edits table data in a grid with staged
changes and SQL preview, and runs SQL in tabs with autocomplete. The interface
structure follows the workflows the TablePro macOS application establishes as
market expectations; every screen here is TableRock's own requirement.

## Screens

- [Connections](connections.md) — list, groups, environment tags, editor,
  test/connect.
- [Workbench](workbench.md) — context bar, sidebar catalog, tabs, status.
- [Data grid](data-grid.md) — browsing, sorting, filtering, columns, paging,
  value inspector.
- [Editing and review](editing.md) — staged changes, highlighting, SQL
  preview, apply/discard.
- [SQL editor](sql-editor.md) — query tabs, autocomplete, execution, results,
  history.
- [Copy and export](copy-export.md) — copy formats, file export, import.
- [Redis screens](redis.md) — key browser, type views, TTL, command editor,
  overview.
- [ClickHouse screens](clickhouse.md) — analytics browsing and honest
  mutations.
- [Native macOS experience](native-macos.md) — Liquid Glass design language
  and platform behavior.

## Two clients, one behavior

Every screen in this spec exists twice:

| | CLI/TUI | Native macOS |
|---|---|---|
| Toolkit | TermRock over Ratatui/Crossterm | SwiftUI shell, AppKit catalog/grid/editor |
| Design language | TermRock theme, cell grid | macOS 26 Liquid Glass |
| Behavior owner | Rust core (in-process) | Same Rust core (embedded via UniFFI) |
| Scope | All product behavior | Presentation and OS integration only |

The TUI ships first (ROADMAP phases 1-11). The native app follows (phases
12-15) and projects the same Rust commands, events, and pages. A screen
specification is not done until its TUI behavior is defined; its native
projection reuses the same Rust contract.

## Rules for these documents

- Describe observable behavior and states, not implementation.
- Sketches show information hierarchy only; they are not geometry sources.
- Every screen lists its empty, loading, error, and unsupported states.
- Engine differences are explicit; no fake cross-engine abstraction.
- Behavior here must trace to the
  [functional parity ledger](../architecture/functional-parity-ledger.md);
  the ledger traces to phases in [`../../ROADMAP.md`](../../ROADMAP.md).
