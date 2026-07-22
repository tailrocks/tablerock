# Evidence 639: native Redis Pub/Sub console

Date: 2026-07-22

## Outcome

`TR-SCR-047` now has a native macOS channel/pattern console over the existing
Rust-owned Redis subscription adapter:

- UniFFI starts a supervised `SUBSCRIBE` or `PSUBSCRIBE` stream without
  exposing a Redis client or Tokio handle to Swift;
- at most four streams run and 256 statuses remain addressable; each status
  retains at most 256 presentation rows while separately counting all received
  messages;
- channel, pattern, and payload bytes remain binary-safe, with invalid UTF-8
  rendered as bounded hexadecimal;
- reconnect/resubscription discontinuity pages become a visible counted gap,
  preserving the engine's ordering before recovered messages;
- cancellation races stream startup and every page wait, drops the dedicated
  stream, is idempotent while pending, and blocks disconnect until terminal;
- native presents channel/pattern selection, starting, waiting, message,
  delivery-gap, failed, cancelled, refresh, cancel, close, and disabled states;
- automatic polling is presentation-only; connection and stream ownership stay
  below UniFFI.

The TUI remains partial because its established console is command-driven
rather than a persistent browser panel.

## Verification

```text
mise exec -- cargo test -p tablerock-ffi --test facade --locked
14 passed

mise exec -- cargo test -p tablerock-ffi --test bridge_real \
  bridge_redis_open_probe_fetch -- --ignored --nocapture
1 passed against Redis 8.8.0

mise exec -- cargo clippy -p tablerock-ffi --all-targets --locked -- -D warnings
green

mise exec -- ./scripts/build-native-app.sh --configuration Release
Built native/dist/TableRock.app
```

The live facade test proves subscribe, independent publish, exact message,
cancel, and ordinary-session reuse. Existing Redis 7.4.9/8.8.0 RESP2/RESP3
matrices prove channel/pattern reconnect with the ordered discontinuity page.
The model and XCUITest suites cover gap presentation and explicit cancellation;
hosted results are attached to the exact completion commit.

## Clean-room provenance

TablePro public material was checked only for the broad existence of a compact
database-tool workbench and stream inspection workflow. No source, tests,
strings, assets, screenshots, layout measurements, colors, or key bindings
were copied. TableRock's supervision, bounds, states, wording, and presentation
were independently designed from repository requirements, the existing Redis
adapter contract, and direct tests.
