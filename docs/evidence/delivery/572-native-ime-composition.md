# 572 — Native IME composition

Date: 2026-07-21

## Decision

The native automation surface now exercises real AppKit marked text rather than
equating Unicode typing with IME composition. A fixture focuses the SQL
`NSTextView`, starts the Japanese marked string `かな`, then mutates the SwiftUI
query model. The representable update must preserve both `hasMarkedText()` and
the composed editor buffer; otherwise the fixture fails visibly.

XCUITest launches that isolated fixture and waits for the stable query-status
proof. This covers the critical presentation race guarded by
`updateNSView`: model publication cannot replace an in-progress input-method
composition.

## Verification

```text
PATH=/Users/donbeave/.cargo/bin:$PATH ./scripts/build-native-app.sh
open -n -F --env TABLEROCK_FIXTURE_IME=1 TableRock.app
```

The strict direct build passes. The application runtime emits:

```text
IME_PROOF_PASSED marked_text_survived_model_update=true
```

The hosted canonical Xcode plan owns the new user-operable XCUITest; hosted
result is pending for this checkpoint.

## Remaining boundary

This proves marked-text preservation for one Japanese composition and a
concurrent presentation update. A manual multilingual input-source matrix,
candidate-window behavior, replacement ranges across selections, dictation,
and accessibility interaction remain open.

## Provenance

Implementation source: TableRock-owned AppKit editor contract and direct test.

TablePro influence: none; this is macOS input-method correctness.

Copied source, tests, identifiers, assets, strings, colors, geometry, layout
measurements, or key bindings: none.
