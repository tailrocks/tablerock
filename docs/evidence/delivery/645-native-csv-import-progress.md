# Evidence 645: progress-aware native CSV apply

Date: 2026-07-22

## Outcome

CSV review tokens now fail closed through one progress-aware apply path:

- ordinary mutation apply rejects CSV review tokens without consuming them;
- start consumes authority before I/O and returns an opaque operation id;
- PostgreSQL checks cancellation at row boundaries and rolls back its active
  transaction; ClickHouse stops at the next row boundary and preserves an
  explicit partial outcome;
- Rust publishes completed/total/applied/conflict/failed counts, a closed phase,
  a terminal summary, and at most 100 row-number-only errors;
- native polls every 100 ms, renders determinate progress, supports cancel,
  exposes partial/cancelled/unknown states, and copies the bounded error list;
- disconnect refuses an active import; shutdown drains or requests cancellation;
  terminal snapshots are explicitly retired.

Evidence 646 supersedes the former 4 MiB/10,000-row boundary by connecting the
constant-memory scanner to fingerprint-bound frozen review and batched engine
apply.

## Verification

```text
mise exec -- cargo test -p tablerock-ffi --test conformance \
  catalog_browse_accepts_only_cached_table_like_nodes --locked
1 passed; CSV cross-purpose rejection, async terminal status, bounded safe
error summary, and retirement covered

mise exec -- cargo test -p tablerock-engine -p tablerock-ffi --locked
passed

mise exec -- cargo clippy -p tablerock-engine -p tablerock-ffi \
  --all-targets --locked -- -D warnings
passed

mise exec -- ./scripts/build-native-app.sh --configuration Release
Swift 6 strict concurrency and warnings-as-errors passed

mise exec -- ./scripts/verify-native-csv-import.sh
PostgreSQL 18.4 native reviewed apply and server-observed two-row outcome passed
```

Model coverage verifies bounded error-copy projection. XCUITest drives shipped
Stage/Apply/progress/outcome and Stage/Apply/progress/cancel/outcome paths;
hosted execution remains required because local Command Line Tools lacks
XCTest.

## Clean-room provenance

TablePro's current public import/export documentation was checked only for the
broad workflow expectations of progress, cancellation, and explicit terminal
error policy. No source, tests, identifiers, strings, assets, screenshots,
layout measurements, colors, or key bindings were copied. TableRock's async
authority, engine cancellation semantics, bounded diagnostics, UI, and tests
derive from plan 016, repository architecture, engine behavior, and direct
tests.
