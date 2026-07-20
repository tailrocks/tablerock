# Evidence 560: bridge real-server readiness

Date: 2026-07-21

## Failure

GitHub Actions run `29779822538` passed the standalone PostgreSQL,
ClickHouse, and Redis bridge cases, then failed the combined three-engine case.
That case treated an accepted ClickHouse TCP connection as query readiness and
performed a one-shot bridge open. ClickHouse accepted TCP before its HTTP query
service was ready.

## Structural correction

The combined conformance path now uses `open_when_ready`, which retries the
actual `TableRockBridge::open` operation with a bounded 8-second deadline.
Raw TCP readiness can no longer authorize the bridge query path.

## Verification

```text
rtk cargo fmt --all --check
PASS

rtk cargo test -p tablerock-ffi --test bridge_real --no-run
PASS

rtk cargo test -p tablerock-ffi --test bridge_real \
  bridge_three_engines_sequential_open_probe -- --ignored --nocapture
PASS — 1 passed, 4 filtered out; 71.60s
```

Remote repetition remains required after push.
