# TableRock

TableRock is a terminal-first, multi-model database workbench for
PostgreSQL, ClickHouse, and Redis. The first product will be a Rust CLI/TUI. A
future native macOS application will use SwiftUI/AppKit over the same Rust core.

Phase 0 research decisions are approved. Implementation is starting; this
repository does not claim a working database client yet.

## Direction

- PostgreSQL, ClickHouse, and Redis only in the first program.
- Rust owns connections, queries, results, edits, history, and safety.
- The TUI uses The Elm Architecture, the independent
  [`termrock`](https://github.com/tailrocks/termrock) component crate, Ratatui,
  and Crossterm as its sole terminal backend.
- Missing neutral TUI components are added and pushed directly to TermRock
  `main`, then consumed at an exact revision.
- Native macOS uses SwiftUI/AppKit over embedded Rust through synchronous UniFFI
  and ships by direct Developer ID notarized distribution.
- Local-only Turso through the Rust `turso` crate stores profiles, preferences,
  intent, and retention-controlled history. `rusqlite` and `libsql` are excluded.
- 1Password is the preferred credential source. Saved plaintext passwords are
  an explicitly dangerous local-testing fallback.
- TablePro, TablePlus, and Zedis are concepts-only references. No source code,
  tests, assets, text, or distinctive layouts are copied or adapted.

Start with [the research map](docs/research/README.md) and
[roadmap](ROADMAP.md).

To execute the complete long-running program with an agent, use the single
canonical prompt:

```text
/goal Follow docs/research/prompt.md
```

## Status

Phase 0 is complete. Phase 1 is in progress: the TermRock prerequisites and
root TEA responsive shell and executable normal/signal terminal paths are
verified. Render-authorized mouse/paste/focus/resize routing and normal, signal,
returned-error, and panic real-PTY restoration paths are implemented. Typed
engine overflow/resync policy remains. Product name, package namespace, and
legal clearance remain subject to final review before public release.

## License

Apache-2.0. See [LICENSE](LICENSE).
