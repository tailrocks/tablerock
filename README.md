# TableRock

TableRock is a proposed terminal-first, multi-model database workbench for
PostgreSQL, ClickHouse, and Redis. The first product will be a Rust CLI/TUI. A
future native macOS application will use SwiftUI/AppKit over the same Rust core.

This repository currently contains research and architecture only. It does not
claim a working database client yet.

## Direction

- PostgreSQL, ClickHouse, and Redis only in the first program.
- Rust owns connections, queries, results, edits, history, and safety.
- The terminal UI consumes the independent Tailrocks TUI component crate.
- A future native macOS client keeps Apple objects in SwiftUI/AppKit and
  database state in Rust.
- 1Password is the preferred credential source. Saved plaintext passwords are
  an explicitly dangerous local-testing fallback.
- TablePro, TablePlus, and Zedis are concepts-only references. No source code,
  tests, assets, text, or distinctive layouts are copied or adapted.

Start with [the research map](docs/research/README.md) and
[roadmap](ROADMAP.md).

## Status

Research phase. Product name, package namespace, and legal clearance remain
subject to final review before the first release.

## License

Apache-2.0. See [LICENSE](LICENSE).
