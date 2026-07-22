# Evidence 642: native typed query parameters

Date: 2026-07-22

## Outcome

`TR-SCR-050` now crosses one Rust-owned named-parameter path:

- Rust inspects up to 64 distinct `:name` placeholders while excluding SQL
  strings, quoted identifiers, comments, dollar quotes, PostgreSQL casts, and
  assignments;
- repeated names reuse one binding while pre-existing positional tokens remain
  unchanged;
- PostgreSQL renders only introduced placeholders as `$n`; ClickHouse renders
  them as engine-correct `{pN:Type}` placeholders;
- bridge requests require an exact unique name set and explicit text, signed
  integer, finite float, Boolean, or NULL type;
- values are bounded, debug-redacted, and passed separately to official
  drivers; no presentation layer can concatenate them into SQL;
- native Run opens a typed sheet before execution, Cancel executes nothing,
  and in-flight cancellation remains reachable.

## Verification

```text
mise exec -- cargo test -p tablerock-core named_params --locked
5 passed

mise exec -- cargo test -p tablerock-ffi --test conformance \
  named_parameters_are_typed_and_never_inlined --locked
1 passed for PostgreSQL and ClickHouse request shapes

mise exec -- cargo test -p tablerock-ffi --test bridge_real \
  bridge_postgres_open_probe_fetch_shutdown --locked -- --ignored --nocapture
1 passed against PostgreSQL 18.4; hostile text returned byte-for-byte as one
bound value

mise exec -- cargo clippy -p tablerock-core -p tablerock-engine \
  -p tablerock-ffi --all-targets --locked -- -D warnings
passed

mise exec -- ./scripts/build-native-app.sh --configuration Release
Built native/dist/TableRock.app
```

Model and XCUITest coverage verify pre-run sheet authority, typed validation,
cancel/run paths, and production result projection. Hosted XCTest/XCUITest and
live ClickHouse replay remain required before final closure.

## Clean-room provenance

TablePro public material was rechecked only for broad database-editor workflow
expectations. No source, tests, identifiers, strings, assets, screenshots,
layout measurements, colors, or key bindings were copied. TableRock's parser,
typed bridge contract, safety behavior, and native presentation derive from
repository requirements, official driver behavior, and direct tests.
