# 544 — Native clock and identity ports

Date: 2026-07-21

## Decision

Plan 021 checkpoint 12 now injects presentation time and identity through
`AppDependencies`. `BridgeModel` no longer reads wall-clock time when staging
or applying reviewed operations and no longer creates window, query-tab,
object-tab, or group-dialog identities directly.

`AppClock` and `AppIdentifierGenerator` are `@MainActor` presentation ports.
This matches their ownership: they create UI correlation facts only. Database
time, operation IDs, result IDs, revisions, and safety truth remain Rust-owned.
Production defaults preserve current `Date`/`UUID` behavior. Deterministic
test implementations prove fixed time and ordered identities without global
state or unchecked `Sendable` conformance.

The direct native builder previously linked one hard-coded feature object.
It now links both feature-module objects explicitly, keeping direct `swiftc`
and SwiftPM on the same application-owned dependency source.

## Verification

```text
./scripts/build-native-app.sh
./scripts/verify-native-accessibility.sh
```

Both strict Swift 6 Release compilation and the launched-app accessibility
runtime gate passed. `AppDependenciesTests` was added to the SwiftPM feature
suite. The local Command Line Tools installation does not provide Swift's
`Testing` module, so that suite remains unexecuted locally; checkpoint 14/16's
full-Xcode CI gate remains required before it becomes release evidence.

## Remaining checkpoint 12 work

- application-owned `WorkbenchBackend` DTO boundary and live adapter;
- scripted scenarios and model-deallocation coverage;
- Keychain namespace, file-panel, and pasteboard ports.

This is a dependency checkpoint, not a visual redesign. TablePro influenced
no expression here. Copied source, tests, identifiers, assets, product text,
geometry, colors, or key bindings: none.
