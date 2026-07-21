# Native typed object filters

Date: 2026-07-22

## Requirement

Native object tabs need independent typed server-filter state. Values must
remain separate from SQL text and cross the driver boundary through native
parameter mechanisms.

## Structural correction

The previous native browse entrypoint discarded `RenderedBrowseSql.parameters`
and could therefore never add safe filters. `BrowsePlan` also emitted only
PostgreSQL `$n` placeholders while ClickHouse requires its own server-parameter
syntax. Presentation work alone could not safely close this gap.

## Delivery

- Added `BrowseDialect`: PostgreSQL retains `$n`; ClickHouse emits typed
  `{pN:Type}` server placeholders. Raw-WHERE fragments that collide with
  generated placeholders fail closed.
- Extended `DriverPageRequest::ClickHouseStatement` with redacted typed
  parameters. The pinned official `clickhouse` 0.15.1 client sends each value
  through `Query::param`; PostgreSQL continues through prepared `query_raw`.
- Added bounded/redacted `BridgeBrowseFilter` intent: 32-filter maximum,
  1,024-byte identifier ceiling, 64-KiB value ceiling, allow-listed operators,
  typed value parsing, identifier validation, and missing/unexpected-value
  rejection. Sort identifiers use the same byte ceiling. General
  arbitrary-query submission still cannot smuggle bridge parameters.
- Added per-object-tab filter drafts and active filters with column/operator/
  value controls, explicit remove/clear actions, stable accessibility IDs, and
  isolated reload behavior. Swift owns only presentation intent; Rust owns
  validation, parameter typing, and SQL rendering.
- Added XCUITest interaction for real add-sort, direction-toggle, typed-filter,
  and active-filter controls. Hosted Xcode execution remains the next evidence
  gate; this document does not claim it passed before that run completes.

## Verification

```text
cargo clippy -p tablerock-engine -p tablerock-ffi -p tablerock-cli \
  --all-targets --locked -- -D warnings
# clean

cargo nextest run -p tablerock-engine --lib -p tablerock-ffi --locked
# 114 passed

cargo nextest run -p tablerock-ffi --test conformance --locked
# 17 passed, including over-limit sort/filter identifiers and filter values

cargo nextest run -p tablerock-engine --test postgres_real --locked \
  -E 'test(=persistent_session_runs_statement_cancel_health_and_reuses_connection)'
# 1 passed

cargo nextest run -p tablerock-engine --test clickhouse_real --locked \
  -E 'test(=persistent_session_runs_statement_health_and_reuses_connection)'
# 1 passed

./scripts/generate-swift-bindings.sh
./scripts/build-native-app.sh
# generated bridge copies synchronized; strict Swift 6 app build passed
```

The FFI conformance suite captures both the statement and parameter count. It
proves values never enter SQL, multi-sort priority remains stable, PostgreSQL
receives `$1/$2`, and ClickHouse receives `{p1:String}/{p2:Int64}`. Real-server
tests prove bound text, integer, and NULL values execute through both clients.

## Remaining scope

Raw-WHERE UI, saved native presets, filter restoration, column-type-driven
editors, and hosted control execution remain open. This checkpoint does not
claim complete grid parity.

## Documentation and provenance

Context7 MCP was unavailable. Implementation syntax was verified against the
exact pinned `clickhouse` 0.15.1 crate source and its `Query::param`
documentation, then against a real ClickHouse 26.3 server. TablePro establishes
only the broad filtering workflow. No TablePro source, tests, identifiers,
product text, assets, screenshots, layout measurements, colors, or key bindings
were copied or translated.
