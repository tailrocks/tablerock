# TableRock

TableRock is a terminal-first, multi-model database workbench for
PostgreSQL, ClickHouse, and Redis. The first product is a Rust CLI/TUI. A
native macOS developer preview uses SwiftUI/AppKit over the same Rust core.

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

The Rust contracts, services, persistence, three real-server adapters, TUI, and
a substantial native macOS workflow slice are implemented. Tested server lines
are PostgreSQL 17.10/18.4, Redis 7.4.9/8.8.0, and ClickHouse 25.8/26.3 LTS.
Unsigned preview packages and an ad-hoc-signed native app are published through
the Homebrew tap. The exact tested boundaries are in the
[support matrix](docs/support-matrix.md).

TableRock does not yet claim complete functional parity or production native
distribution. Visible gaps remain in the
[functional parity ledger](docs/architecture/functional-parity-ledger.md).
Developer ID signing, notarization, stapling, and clean-machine release proof
remain externally blocked on signing authority.

Product name, package namespace, and legal clearance remain subject to final
review before public release.

Details: [roadmap](ROADMAP.md) · [evidence index](docs/evidence/README.md)

## Safe support facts

Generate the local safe-schema support manifest without opening the terminal UI:

```bash
tablerock --support-bundle > tablerock-support.txt
```

The command reports only schema version, TableRock version, and closed platform
facts. The running native bridge additionally retains bounded closed engine
diagnostics and terminal outcomes, and Settings can export that state through a
save panel using Rust's atomic writer. Neither path reads logs, profiles,
history, SQL, database values, endpoints, hostnames, or credentials. Inspect
the text before sharing it. Engine-specific safe codes, a long-lived TUI
collector, and crash-report sanitization remain visible Phase 15 work.

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
