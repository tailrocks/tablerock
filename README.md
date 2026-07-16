# TableRock

TableRock is a terminal-first, multi-model database workbench for
PostgreSQL, ClickHouse, and Redis. The first product will be a Rust CLI/TUI. A
future native macOS application will use SwiftUI/AppKit over the same Rust core.

Phase 0 research decisions are approved. Phase 2 driver feasibility is in
progress with real-server PostgreSQL, ClickHouse, and Redis evidence; this
repository does not yet claim a complete workbench.

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

Phases 0 and 1 are complete. The TermRock prerequisites and
root TEA responsive shell and executable normal/signal terminal paths are
verified. Render-authorized mouse/paste/focus/resize routing and normal, signal,
returned-error, and panic real-PTY restoration paths are implemented. Typed
bounded ingress overflow/resync policy is implemented and audited. Phase 2 is
in progress with dependency-minimal core identity/revision, bounded owned-value,
redaction, explicit per-engine capability, immutable columnar page,
live-session operation lifecycle, safe diagnostic, typed command-envelope, and
versioned secret-source and profile-property policy tracers. Ordinary literal
password/private-key material is structurally rejected while every property
remains sourceable.
Immutable profile connection snapshots add stable identity/revision, all-engine host/port
readiness, validated TLS state, two-mode safety, and finite limits.
The baseline profile aggregate adds saved/temporary disposition, bounded
organization/preferences, redaction, and monotonic replacement validation.
Local state now has a bounded offline checkpointed backup, strict SHA-256
manifest, tamper detection, and absent-target independently verified restore;
operator replacement and remaining storage fault/deployment gates stay open.
Local persistence now has an isolated default-features-disabled Turso 0.7
worker, bounded serialized commands, sequential documented migrations, and
normalized process-local single ownership plus interrupted-migration recovery
and abrupt-process reopen evidence; profile persistence remains gated.
Saved-profile create accepts only core-issued persistence tokens and atomically
stores the complete version-1 aggregate. Strict transactional lookup now
reconstructs all three engines and all six source kinds through core validators;
transactional revision compare-and-swap replaces complete aggregates without a
last-writer-wins bypass. Revision-CAS deletion removes only profile-owned rows;
a least-data, keyset-paginated summary list is capped at 100 items. Exact engine,
favorite, group, and tag filters use scope-bound cursors. Unicode-normalized
name/group/tag search is bounded by a 10,000-profile durable capacity. List
summaries expose validated literal endpoints or unresolved secret markers.
Bounded adapter rows now assemble through one checked immutable result-page
path. The private PostgreSQL adapter proves driven ownership, bounded pages,
verified custom-root TLS, independent server-name verification, client
certificate identity, plaintext downgrade rejection, and TLS cancellation on
PostgreSQL 17.10 and 18.4; the remaining driver matrix stays Phase 2 work.
The Redis supported-line RESP2/RESP3 matrix also proves bounded per-command
pipeline partial failures, `MULTI`/`EXEC` no-rollback behavior, and exact
missing/persistent/finite-millisecond key TTL facts; broader Redis TLS,
command-family, Pub/Sub, reviewed TTL mutation, timeout, and reconnect evidence
remains open.
Product name, package namespace, and legal clearance remain subject to final
review before public release.

## License

Apache-2.0. See [LICENSE](LICENSE).
