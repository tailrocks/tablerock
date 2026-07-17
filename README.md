# TableRock

TableRock is a terminal-first, multi-model database workbench for
PostgreSQL, ClickHouse, and Redis. The first product is a Rust CLI/TUI. A
future native macOS application will use SwiftUI/AppKit over the same Rust
core.

## Direction

- PostgreSQL, ClickHouse, and Redis only in the first program.
- Rust owns connections, queries, results, edits, history, and safety.
- The TUI uses The Elm Architecture, the independent
  [`termrock`](https://github.com/tailrocks/termrock) component crate, Ratatui,
  and Crossterm as its sole terminal backend.
- Missing neutral TUI components are added and pushed directly to TermRock
  `main`, then consumed at an exact revision.
- Native macOS uses SwiftUI/AppKit over embedded Rust through synchronous
  UniFFI and ships by direct Developer ID notarized distribution.
- Local-only Turso through the Rust `turso` crate stores profiles, preferences,
  intent, and retention-controlled history. `rusqlite` and `libsql` are
  excluded.
- 1Password is the preferred credential source. Saved plaintext passwords are
  an explicitly dangerous local-testing fallback.
- TablePro, TablePlus, and Zedis are concepts-only references. No source code,
  tests, assets, text, or distinctive layouts are copied or adapted.

## Status

Phases 0 and 1 are complete. Phase 2 (Rust service foundation) is in progress:
core contracts, application services, profiles, and persistence are proven, and
all three drivers hold real-server evidence — PostgreSQL 17.10/18.4, Redis
7.4.9/8.8.0, ClickHouse 25.8/26.3 LTS. This repository does not yet claim a
complete workbench.

Product name, package namespace, and legal clearance remain subject to final
review before public release.

Details: [roadmap](ROADMAP.md) · [evidence index](docs/evidence/README.md)

## Documentation

Start with the [documentation map](docs/README.md): the
[product specification](docs/product/README.md) defines every screen, the
[architecture](docs/architecture/fixed-decisions.md) defines how, and the
[evidence index](docs/evidence/README.md) records what is proven. To execute
the complete long-running program with an agent, use the single canonical
prompt:

```text
/goal Follow docs/prompt.md
```

## License

Apache-2.0. See [LICENSE](LICENSE).
