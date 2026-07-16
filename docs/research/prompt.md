# Research Brief

## Mission

Build TableRock as a focused PostgreSQL, ClickHouse, and Redis workbench. Ship a
Rust CLI/TUI first and a direct-notarized native SwiftUI/AppKit macOS application
later over the same embedded Rust engine.

## Fixed implementation path

- Rust owns profiles, secrets policy, database adapters, sessions, catalogs,
  queries, results, edits, history, persistence, redaction, and safety.
- The TUI uses Ratatui's The Elm Architecture, TermRock for every reusable
  interactive primitive, and Crossterm as the sole terminal backend/input.
- Missing neutral components are implemented and pushed directly to TermRock
  `main`, then consumed from an exact revision; no branch or pull request.
- `tokio-postgres`, official `ClickHouse/clickhouse-rs`, and `redis-rs` are the
  only database clients.
- `sqlparser` is the SQL parser and bundled SQLite through `rusqlite` is the
  local persistence store.
- SSH tunneling uses a Rust `russh` adapter; cloud-provider proxy/identity
  integrations are excluded.
- The macOS application embeds Rust through synchronous coarse UniFFI, uses
  SwiftUI for application structure and AppKit outline/table/text controls, and
  ships through Developer ID/hardened runtime/notarization/stapling.
- All work is direct, forward-only commits on `main` with no branches or pull
  requests.

## In scope

- saved, temporary, organized, URL-imported profiles;
- 1Password, prompt-on-connect, Keychain, environment, and acknowledged
  dangerous plaintext secret sources under one Rust SecretSource contract;
- TLS, Test, connect/disconnect/reconnect and context switching;
- catalog, table/key browsing, structure, typed values, inspectors, sorting,
  filtering, paging, copy, and column preferences;
- PostgreSQL/ClickHouse SQL and Redis command editing, completion, files,
  parameters, formatting, history, saved queries, explain, and cancellation;
- staged edits, review, conflict/partial-outcome/safety semantics;
- imports, exports, applicable administration, backup/restore, relationships,
  roles, startup actions, and optional Vim behavior as sequenced by the ledger;
- complete terminal and native macOS behavior over the same Rust service.

## Excluded

- databases other than PostgreSQL, ClickHouse, and Redis;
- cloud-provider proxy and identity integrations;
- third-party driver marketplace, iOS/iPadOS, team licensing/commerce;
- AI assistance, natural-language query generation, MCP, or external-agent
  database access;
- copied TablePro/TablePlus/Zedis source or expression;
- helper daemon, local RPC, WebView, manual C ABI, Mac App Store distribution,
  second SQL parser, or second TUI component framework.

## Evidence rule

External claims use primary sources and dates. TablePro, TablePlus, and Zedis
establish broad product workflows only. Implementation comes from this brief,
official database/client/platform/library documentation, and direct tests.
