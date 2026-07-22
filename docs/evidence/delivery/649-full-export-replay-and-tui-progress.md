# Evidence 649: full object-export replay and TUI progress

## Claim

Object-browse results retain a bounded in-memory replay keyed by opaque result
identity. The replay is produced beside the ordinary Rust `BrowsePlan`: same
quoted identifiers, typed filters and parameters, validated raw WHERE, and
ordered sort keys. Its full-export form intentionally omits only interactive
`LIMIT/OFFSET`; database streaming remains bounded by 500-row, 8 MiB pages and
64 KiB cells. Native passes result identity, revision, format, and destination;
Swift cannot manufacture SQL or alter the frozen typed plan.

TUI full-result export owns a dialog state with running, cancel-requested,
completed, cancelled, and failed phases; rows, bytes, destination, safe summary,
Cancel, and Close. The executor registers one cancellation channel per request,
emits progress after each page, interrupts a pending page with `tokio::select!`,
dispatches driver cancellation, and relies on the shared atomic writer to
remove incomplete output. The former interactive 10,000-row cap no longer
truncates file export.

TR-SCR-054 is proven for TUI and implementation-complete for native. Native
remains `partial` only because hosted XCUITest replay of the progress/cancel
sheet is still unavailable locally.

## Verification

```text
mise exec -- cargo test -p tablerock-engine -p tablerock-ffi --test conformance -p tablerock-tui -p tablerock-cli --lib --locked
mise exec -- cargo clippy -p tablerock-engine -p tablerock-ffi -p tablerock-tui -p tablerock-cli --all-targets --locked -- -D warnings
mise exec -- ./scripts/build-native-app.sh --configuration Release
mise exec -- cargo test -p tablerock-core --test screen_manifest --locked
```

Results: engine/bridge/TUI/CLI suites passed. Conformance proves catalog replay
uses an unbounded SQL shape while preserving typed execution, plus atomic export
completion. TUI reducer test proves start/progress/cancel/terminal/close flow;
render test proves the shipped dialog exposes progress and Cancel. Clippy and
direct Swift 6 release build passed.

## Clean-room provenance

TablePro public material established only the broad workflow expectation for
full export, progress, cancellation, and terminal outcomes. No TablePro source,
tests, identifiers, product text, assets, colors, geometry, layout measurements,
or key bindings were read or copied. Typed replay, dialog expression, copy, and
tests are TableRock-owned.
