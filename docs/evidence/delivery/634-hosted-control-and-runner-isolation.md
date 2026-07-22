# Evidence 634: hosted control and runner isolation

Date: 2026-07-22

## Outcome

Four native controls that existed in the model but failed canonical XCUITest
operation now expose deterministic macOS accessibility actions:

- result cells use the standard `NSButton` target/action path, including AX
  press, before projecting selection into the value inspector;
- external-URL authority actions live in the sheet body instead of a toolbar
  group that could collapse out of the accessibility tree;
- Quick Switcher owns an identified search text field instead of relying on
  whichever `.searchable` field XCUITest enumerates first;
- Explain proof selects the identified application menu command and waits for
  its connected-session enablement before invoking it.

Canonical run 29888579521 then proved the remaining common cause: SwiftUI
`FocusedValue` retained the command capability booleans from before connection,
so Explain and PostgreSQL Activity remained disabled and the Quick Switch
shortcut could resolve without an active scene target. Commands now receive a
live `BridgeModel` through `focusedSceneValue` and evaluate capability when the
menu asks. Result buttons additionally implement direct mouse and AX-press
activation instead of relying on one NSControl dispatch path.

The repeated Velnor PTY timeout came from fleet version 0.1.58 reusing a
persistent Cargo target whose `tablerock` test binary predated the checked-out
source. Velnor 0.1.109 fixes this bug structurally by including source revision
in persistent-target identity. Serializing real-server integration after the
Rust job would only hide stale executable reuse and reduce valid parallelism,
so both jobs remain independent. The exact stress test passed 20 consecutive
nextest runs on the current source.

## Verification

```text
mise exec -- swift build --package-path native -c release
Build complete

mise exec -- cargo nextest run -p tablerock-cli --test pty_lifecycle --locked
20 consecutive runs passed; 4 tests per run

GitHub-hosted CI run 29886270653, Format/lint/test
success

Canonical Native Checkpoint run 29888579521
23 XCUITests executed; four failures isolated to stale focused-command state
and result-cell activation; structural fixes applied in the following checkpoint

mise exec -- cargo clippy -p tablerock-cli -p tablerock-engine --tests -- -D warnings
green

mise exec -- actionlint .github/workflows/ci.yml
green

mise exec -- cargo fmt --all --check
green

git diff --check
green
```

Local Swift XCTest remains unavailable because this host's Command Line Tools
SDK does not provide `XCTest`. Canonical Xcode/XCUITest and the upgraded Velnor
lane remain required after push.

## Primary sources

- Apple controls and XCUITest behavior are verified by the repository's
  canonical hosted Xcode project rather than inferred from local SDK behavior.

## Clean-room provenance

TablePro public documentation was checked only for broad workflow existence:
external links require confirmation, connection/query switching is searchable,
and Explain is a query-workbench action. No source, tests, strings, assets,
geometry, measurements, colors, layout, or key bindings were copied. TableRock
control placement, identifiers, Rust ownership, authority rules, and tests are
independently defined from repository requirements and direct failure evidence.
