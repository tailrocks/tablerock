# TableRock

TableRock is a native macOS and terminal database workbench for PostgreSQL,
ClickHouse, and Redis. Both clients run in-process on one Rust core that owns
database behavior, safety, persistence, and bounded result handling.

The macOS app is a SwiftUI/AppKit application for macOS 26 with a Liquid Glass
shell and native catalog, grid, and editor controls. The terminal client is a
Rust TUI built with The Elm Architecture, TermRock, Ratatui, and Crossterm.

## What is implemented

- PostgreSQL, ClickHouse, and Redis profiles, TLS/mTLS, SSH tunnels, connection
  testing, reconnect policy, groups, environment markers, and safety modes.
- Prompt, 1Password, environment, native Keychain, and explicitly acknowledged
  local-testing secret sources. Resolved credentials never enter durable state.
- Engine-native catalogs, object and query tabs, Redis key/type views, typed
  paged results, sorting, filtering, column layouts, value inspection, copy,
  atomic streaming export, and streaming CSV import.
- Query history, saved queries, SQL files, parameters, completion, find/replace,
  cancellation, session-intent restoration, and native multi-window workflows.
- Reviewed, engine-aware writes below presentation, including PostgreSQL row
  updates and structure/table operations where stable identity and capability
  facts are proven.
- PostgreSQL activity, relationships, roles, maintenance, and backup/restore;
  Redis INFO and Pub/Sub; ClickHouse-specific structure and operation handling.

The product engine boundary is exactly PostgreSQL, ClickHouse, and Redis. A
bundled local SQLite sample exists only as a deterministic development and UI
fixture; it is not a supported connection engine.

## Architecture

| Layer | Responsibility |
|---|---|
| `tablerock-core` | Stable IDs, values, commands, pages, revisions, safety, and redaction |
| `tablerock-engine` | PostgreSQL, ClickHouse, Redis, SSH, catalog, query, and mutation adapters |
| `tablerock-persistence` | One serialized local-only Turso actor for profiles, preferences, intent, and bounded history |
| `tablerock-tui` / `tablerock-cli` | TEA terminal presentation and process lifecycle |
| `tablerock-ffi` | Coarse synchronous UniFFI facade over the Rust application service |
| `native/` | SwiftUI shell plus AppKit catalog, result-grid, and editor presentation |

The native app embeds Rust; it does not use a daemon, local RPC service,
WebView, or parallel Swift database implementation. Results cross presentation
boundaries in immutable bounded pages rather than per-cell calls.

## Run from source

The workspace pins Rust 1.97. Native development requires macOS 26 and full
Xcode 26.6.

Terminal client:

```bash
cargo run --release -p tablerock-cli
```

Local native development app:

```bash
./scripts/build-native-app.sh
open native/dist/TableRock.app
```

That native bundle is ad-hoc signed for local verification. Developer ID
signing, notarization, stapling, and clean-machine distribution use the
operator-gated release workflow and are not implied by the local build.

Common Rust gates:

```bash
mise run fmt
mise run ci
mise run lint
mise run test
```

Real-server suites use Docker containers. The native checkpoint and release
commands are documented in [`native/README.md`](native/README.md).

## Project status

The Rust services, three real-server adapters, TUI, and selected production
native workbench are implemented. Current repository evidence exercises
PostgreSQL 17.10/18.4, ClickHouse 25.8.28.1/26.3.17.4 LTS, and Redis 7.4.9/8.8.0.
See the [tested support matrix](docs/support-matrix.md) for exact platform and
runtime boundaries.

TableRock does not yet claim complete functional parity or a Developer ID
notarized native release. Remaining capability and replay gaps stay visible in
the [functional parity ledger](docs/architecture/functional-parity-ledger.md).
Product naming and legal clearance also remain subject to final review before
public production distribution.

## Safe support facts

Generate the local safe-schema support manifest without opening the terminal
UI:

```bash
tablerock --support-bundle > tablerock-support.txt
```

The command reports only schema version, TableRock version, and closed platform
facts. Native Settings can also export bounded closed engine diagnostics and
terminal outcomes. Neither path reads logs, profiles, history, SQL, database
values, endpoints, hostnames, or credentials. Inspect the text before sharing
it.

## Documentation

Start with the [documentation map](docs/README.md). The
[product specification](docs/product/README.md) defines operator behavior, the
[architecture decisions](docs/architecture/fixed-decisions.md) define system
ownership, the [roadmap](ROADMAP.md) tracks phases, and the
[evidence index](docs/evidence/README.md) records what has been proven.

TablePro, TablePlus, and Zedis may inform commonplace workflows and interface
composition under the repository's
[clean-room policy](docs/architecture/clean-room-reference.md). TableRock does
not copy their implementation, tests, branding, proprietary assets, or source
expression.

## License

Apache-2.0. See [LICENSE](LICENSE).
