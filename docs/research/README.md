# TableRock Research

Research conducted on 2026-07-15 for a terminal-first database workbench.

## Fixed direction

- Standalone Tailrocks product, not a `jackin❯` feature.
- PostgreSQL, ClickHouse, and Redis only.
- Rust core and CLI/TUI first.
- Future native SwiftUI/AppKit macOS client over the same Rust core.
- Official `ClickHouse/clickhouse-rs` client.
- `redis-rs/redis-rs` for Redis.
- 1Password-first connection setup; plaintext secrets remain dangerous.
- Concepts-only use of TablePro, TablePlus, and Zedis.

## Map

- [Research brief](prompt.md)
- [Vision and scope](00-vision-and-scope.md)
- [Clean-room reference policy](01-clean-room-reference.md)
- [Workflow inventory](02-workflow-inventory.md)
- [Database capability model](03-database-capabilities.md)
- [Redis reference analysis](04-redis-reference-zedis.md)
- [Product identity](05-product-identity.md)
- [Rust core architecture](10-rust-core-architecture.md)
- [Terminal experience](11-terminal-experience.md)
- [Native macOS path](12-native-macos-path.md)
- [Dependency evaluation](20-dependency-evaluation.md)
- [Delivery plan](30-delivery-plan.md)
- [Open decisions](31-open-decisions.md)

## Architecture headline

```text
tablerock-cli
  |- tablerock-tui ------> tailrocks-tui
  `- tablerock-engine
          |
          v
     tablerock-core

tablerock-engine/drivers/{postgres, clickhouse, redis}
```

Rust owns authoritative database state. Presentation owns focus, layout,
keyboard/mouse input, and rendering. Results cross boundaries in immutable
batches/pages rather than per cell.
