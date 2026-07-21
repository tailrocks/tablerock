# 580 — Native temporary-connect XCUITest

Date: 2026-07-21

## Behavior

The canonical app UI suite launches with an isolated scripted backend, finds
the stable direct-connect control, clicks it with the default reviewed form,
and waits for the real status surface to report a connected session. This
proves user-operable temporary connection presentation above the already-tested
backend boundary; it does not claim real database protocol semantics.

The fixture uses a unique temporary application root and persists no profile or
credential. Rust real-server suites remain authoritative for connection and
database behavior.

## Verification

`swiftc -parse` and `git diff --check` pass locally. Full XCUITest execution is
pending on the hosted Xcode 26.6 checkpoint after push.

## Provenance

Implementation source: TableRock's native connection requirements and stable
automation surface.

TablePro influence: broad temporary-connection workflow only. No source,
tests, identifiers, assets, product text, screenshots, layout measurements,
colors, or key bindings were copied or translated.
