# Phase 2 PostgreSQL Notice Detail and Hint Evidence

Date: 2026-07-17

## Decision

PostgreSQL notice `DETAIL` and `HINT` are optional bounded fields, not text
concatenated into the primary message. Each independently preserves presence,
UTF-8-safe truncation, and original byte length. `Debug` exposes only optional
byte lengths and never field content.

The same 1,024-byte cap applies independently to message, detail, and hint. One
server-controlled field therefore cannot consume unbounded memory or hide which
protocol field carried the content.

## Evidence

The PostgreSQL 17.10/18.4 notice matrix raises one notice with exact message,
detail, and hint. It requires all three values, complete truncation states, and
absence of every value from formatted `Debug`. The separate long-message notice
has no detail or hint and preserves both fields as absent.

Existing 64-entry queue and explicit overflow evidence remains green, proving
the additional fields do not change capacity or delivery behavior.

This closes bounded notice detail/hint ownership. Schema/table/column/position
fields, LISTEN notifications, service/UniFFI/UI projection, persistence policy,
multiple statements, COPY, reconnect, and ambiguous writes remain open.

## Safety contract

- Optional fields remain distinct; no presentation-ready concatenation occurs.
- Each field has an independent byte bound and truncation truth.
- Absent fields remain absent, never empty-string substitutes.
- Message, detail, and hint remain absent from `Debug`, errors, and default logs.

## Provenance

External concepts: PostgreSQL notice protocol fields
Public source: <https://www.postgresql.org/docs/current/protocol-error-fields.html>
TableRock requirements: research 01, 06, 10, 14, 20, 30, 31, 32, 159
Implementation source: TableRock-owned bounded notice contract and real-server
fixture
Copied code/assets/text: none
