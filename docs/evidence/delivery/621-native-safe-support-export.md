# Native safe support export

Date: 2026-07-22

## Behavior

- Settings exports through the injected file-panel capability into an isolated
  operator-selected path.
- Scripted success mode writes only the closed support-bundle schema, enabling
  shipped-surface automation without touching user data or asserting live Rust
  semantics.
- Canonical XCUITest now clicks Export, observes completion, reads the produced
  file, requires schema and diagnostic-count fields, and rejects password or
  statement field names.
- Settings exposes the completion result as a stable accessibility value.

Production remains backed solely by Rust `export_support_bundle`; Swift owns
file-panel presentation and outcome display only.

## Local verification

```text
(cd native && swift build -c release)
# production package compiled

xcrun swiftc -parse native/Tests/TableRockAppUITests/TableRockAppUITests.swift
# canonical UI suite parsed

git diff --check
# clean
```

Local XCTest remains unavailable under the installed Command Line Tools-only
developer directory. Exact-main hosted Xcode proof remains required.

## Provenance

No external product source, test, identifier, product text, asset, screenshot,
layout measurement, color, or key binding influenced this checkpoint. It
implements TableRock plan 021's safe diagnostic-export requirement from
TableRock-owned contracts and tests.
