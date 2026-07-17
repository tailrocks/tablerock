# Phase 2 PostgreSQL Typed Parameter Evidence

Date: 2026-07-17

## Decision

PostgreSQL parameters are encoded by `tokio-postgres` and remain behind the
driver adapter. Stable TableRock contracts expose only the resulting bounded
typed page; they never expose `ToSql`, statements, rows, or client types.

The Phase 2 tracer uses fixed TableRock-owned values and SQL. Arbitrary user SQL
and parameter collections belong to the reviewed execution plan, where count,
type, and byte bounds must be validated before I/O.

## Evidence

Official PostgreSQL 17.10 and 18.4 Testcontainers fixtures prepare one statement
with four positional parameters and execute it through `query_raw`: UTF-8 text
with a multibyte character, a near-minimum signed `int8`, binary bytes containing
NUL and `0xff`, and boolean false.

The scalar assertions require four exact engine types in one final row and exact
normalized value kinds/bytes; research 158 expands that row to six columns. A following page request returns
end-of-stream. The same session first completes the broader typed-value matrix
and then shuts down cleanly, proving parameter ownership does not leak beyond
stream construction.

This closes PostgreSQL typed parameter transport for the Phase 2 feasibility
gate. Research 158 subsequently closes NULL and integer-array parameters. Public
parameter-plan bounds, composites, prepared statement lifecycle/cache policy,
notices, multiple statements, COPY,
connection loss/reconnect, ambiguous writes, presentation, and UniFFI encoding
remain open.

## Safety contract

- SQL and parameter data are never concatenated.
- Parameter values never enter default logs, errors, history, or diagnostics.
- Database-client parameter types remain adapter-private.
- Result values retain page and cell byte bounds after server decoding.
- Failed parameter execution is never automatically replayed.

## Provenance

External concepts: PostgreSQL extended-query parameters and tokio-postgres
parameter encoding
Public sources: <https://docs.rs/tokio-postgres/latest/tokio_postgres/struct.Client.html#method.query_raw>
and <https://www.postgresql.org/docs/current/protocol-flow.html#PROTOCOL-FLOW-EXT-QUERY>
TableRock requirements: research 01, 06, 10, 14, 20, 30, 31, 32, 81, 87, 136
Implementation source: TableRock-owned fixed probes and bounded page decoder
Copied code/assets/text: none
