# Plan 014 residual — ClickHouse ExecuteSql + query_id status

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `execute_sql` dispatches by engine (PG / CH / Redis) | done |
| CH `ClickHouseStatement` with client `tr-{token}` query_id | done |
| CH rejects bound `$n` parameters (honest fail) | done |
| Redis free SQL rejected with explicit message | done |
| `GridPage.server_query_id` + status line `qid …` | done |
| Unit: status line + grid page retain qid | done |

## Decision

Workbench SQL previously always built `PostgreSqlStatement`, so ClickHouse
sessions failed with engine mismatch. Route by `session.engine()`, assign a
stable client query id for CH cancel, and project it on the first grid page
into the status bar. Full HTTP progress events remain residual.

## Evidence

```text
cargo test -p tablerock-tui --lib status_line_includes_query_id
cargo test -p tablerock-tui --lib grid_page_fills
cargo test -p tablerock-cli --lib
```

## Remaining work

- ClickHouse browse-plan bind/literal path (parameters today fail closed).
- ~~Progress/read_bytes into status bar~~ (closed: evidence 320).
