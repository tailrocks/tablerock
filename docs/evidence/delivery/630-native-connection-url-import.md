# Evidence 630: native connection URL import

Date: 2026-07-22

## Outcome

Native connection list now exposes an Import URL action. Input stays in a
secure field and crosses the application-owned backend seam to the shared Rust
`parse_connection_url` policy. Successful parsing opens the existing complete
profile editor as a mandatory review step; no parse operation connects, saves,
or resolves credentials.

Passwords carried in URLs default to the injected macOS Keychain path. Engine,
endpoint, default context, user, TLS intent, safety mode, and password source
remain editable before Save. Rust rejects oversized, malformed, control-byte,
and hostile-scheme input as typed `connection-url` errors.

## Tests

- Rust facade test proves PostgreSQL field/TLS/password projection and hostile
  `javascript:` rejection through the shipped UniFFI object.
- XCUITest enters a URL through the shipped secure field, activates Review,
  verifies projected fields, and proves Save is available only in the separate
  editor.
- Generated Swift bindings were regenerated from the Rust API. Release Swift
  app compilation remains green.

Local commands:

```text
mise exec -- cargo test -p tablerock-ffi --test facade connection_url_becomes_unsaved_review_draft
1 passed

./scripts/generate-swift-bindings.sh
generated bindings updated

mise exec -- cargo clippy -p tablerock-ffi --all-targets -- -D warnings
green

cd native && rtk swift build -c release
ok (build complete)
```

Hosted XCUITest remains required after push. Local `swift test` cannot load
XCTest because this host has Command Line Tools only, with no full Xcode app.

## Clean-room provenance

TablePro public feature overview was reviewed only to confirm connection-import
and credential-review workflow classes. No source, tests, strings, assets,
layout measurements, colors, or key bindings were read or copied. TableRock's
UI, identifiers, policy, Rust parser, and tests derive from repository product
requirements and platform-owned controls.
