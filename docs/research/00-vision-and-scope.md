# Vision And Scope

## Problem

Agent-driven development increases the amount of software work running without
continuous human attention. Humans still need a fast, inspectable surface for
understanding project databases, validating changes, and intervening safely.
Existing polished clients are primarily desktop applications. TableRock starts
with the terminal so it works beside agents, containers, remote shells, and
normal CLI workflows.

## Product

TableRock is a standalone Tailrocks database workbench. It is not part of the
agent orchestrator, observability backend, CI runner, or project command tool.
It may integrate with those products through commands and OpenTelemetry, but it
owns its profiles, credentials, sessions, results, and safety policy.

The first complete loop is:

1. Create a PostgreSQL, ClickHouse, or Redis profile.
2. Map connection properties from a 1Password item or choose another explicit
   source, configure TLS and safety, then test.
3. Connect and select database/schema/logical database as supported.
4. Browse objects or Redis keys.
5. Inspect typed data in a viewport-backed grid or value-specific view.
6. Execute SQL or Redis commands, stream bounded results, and cancel.
7. Stage supported edits, review the exact parameterized operation/commands,
   then explicitly apply or discard.

## Engine scope

### PostgreSQL

Transactional relational browsing and editing, rich schemas and types,
prepared parameters, server cancellation, and conflict-aware changes.

### ClickHouse

Analytical browsing and large streaming results using the official Rust client.
Inserts and asynchronous mutations are presented honestly rather than modeled
as PostgreSQL transactions.

### Redis

Logical databases, cursor-based key discovery, namespace projection, typed
values, TTLs, commands, and a bounded current `INFO` overview. Redis is never
forced into one relational abstraction.

## Safety baseline

- New profiles default to Confirm writes and can be Read only.
- 1Password is the preferred stored-secret source.
- Resolved values exist only during Test/Connect and never enter snapshots.
- Plaintext passwords require explicit dangerous local-testing acknowledgement
  and a persistent warning.
- Results, timeouts, and memory are bounded.
- Writes use parameters/typed command plans and explicit review.
- Reconnect never repeats an ambiguous write automatically.
- SQL text and cell values are absent from default telemetry.

## Product boundary

TableRock consumes the independent `tailrocks-tui` component project. It never
imports `jackin❯` product crates. A future general secret crate is considered
only after a second consumer proves a stable neutral contract.
