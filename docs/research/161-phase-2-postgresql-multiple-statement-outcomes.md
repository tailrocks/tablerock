# Phase 2 PostgreSQL Multiple-Statement Outcome Evidence

Date: 2026-07-17

## Decision

PostgreSQL multi-statement execution produces one ordered outcome per completed
statement. The Phase 2 feasibility contract records only ordinal, honest
`Query`/`Command` kind, and PostgreSQL row count. It does not expose
`SimpleQueryMessage`, command text, or text-decoded row values, and it is not the
final cross-engine result-tab contract.

The tracer uses fixed TableRock-owned SQL and a hard four-outcome bound. Public
arbitrary SQL requires parser-owned statement boundaries, reviewed execution,
bounded result pages per statement, cancellation, partial-failure truth, and
history/redaction before this shape can graduate.

## Evidence

Official PostgreSQL 17.10 and 18.4 Testcontainers fixtures stream a fixed batch
through `simple_query_raw` and require these ordered outcomes:

1. temporary table creation: command, zero rows;
2. two-row insert: command, two rows;
3. one-row update: command, one row; and
4. ordered select: query, two rows.

`RowDescription` establishes query kind; each `CommandComplete` closes exactly
one ordinal. Row values are intentionally ignored in this tracer because the
existing extended-query binary stream remains the only typed result path.

This closes ordered PostgreSQL multiple-statement outcome feasibility. Typed
result pages per statement, parser boundaries, transaction/partial-failure
semantics, statement-count and SQL-byte bounds, cancellation, notices per
statement, service/result-store integration, history, UI/UniFFI, and arbitrary
user execution remain open.

## Safety contract

- Multi-statement row values never enter the simple-query text path.
- Statement ordering and row counts come from protocol completion messages.
- A fifth or missing outcome fails the fixed tracer.
- SQL text and values remain absent from stable outcomes, errors, and logs.
- This tracer cannot execute presentation-supplied SQL.

## Provenance

External concepts: PostgreSQL simple-query multiple-statement protocol messages
Public sources: <https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-SIMPLE-QUERY>
and <https://docs.rs/tokio-postgres/0.7.18/tokio_postgres/enum.SimpleQueryMessage.html>
TableRock requirements: research 01, 06, 10, 14, 20, 30, 31, 32, 77, 112, 118
Implementation source: TableRock-owned fixed tracer and bounded outcome facts
Copied code/assets/text: none
